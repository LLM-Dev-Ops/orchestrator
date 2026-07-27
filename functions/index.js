'use strict';

const crypto = require('crypto');
const { AGENT_CONTRACTS } = require('./contracts');

// =============================================================================
// orchestrator-agents Cloud Function
// Entry point: handler
// Runtime: nodejs20
//
// This function does NOT orchestrate. It authenticates, applies CORS, publishes
// contracts, and proxies every agent request to the Rust engine on Cloud Run,
// which owns all orchestration logic. See ADR-0001.
// =============================================================================

const SERVICE_NAME = 'orchestrator-agents';
const SERVICE_VERSION = '0.2.0';
const BASE_URL = 'https://us-central1-agentics-dev.cloudfunctions.net/orchestrator-agents';

const AGENTS = ['workflow', 'scheduler', 'dependencies', 'retry', 'parallel', 'state-machine', 'swarm'];

const AGENT_DISPLAY_NAMES = {
  workflow: 'Workflow Orchestrator Agent',
  scheduler: 'Task Scheduler Agent',
  dependencies: 'Dependency Resolver Agent',
  retry: 'Retry & Recovery Agent',
  parallel: 'Parallelization Agent',
  'state-machine': 'State Machine Agent',
  swarm: 'Swarm Coordinator Agent',
};

const AGENT_ROUTES = {
  workflow: '/v1/orchestrator/workflow',
  scheduler: '/v1/orchestrator/scheduler',
  dependencies: '/v1/orchestrator/dependencies',
  retry: '/v1/orchestrator/retry',
  parallel: '/v1/orchestrator/parallel',
  'state-machine': '/v1/orchestrator/state-machine',
  swarm: '/v1/orchestrator/swarm',
};

// =============================================================================
// Upstream engine configuration
// =============================================================================

const ENGINE_URL = process.env.ORCHESTRATOR_ENGINE_URL;
const ENGINE_TIMEOUT_MS = Number(process.env.ORCHESTRATOR_ENGINE_TIMEOUT_MS || 30000);

// Set only for local development against an engine that does not require auth.
const SKIP_AUTH = process.env.ORCHESTRATOR_ENGINE_SKIP_AUTH === 'true';

const METADATA_IDENTITY_URL =
  'http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/identity';

// Identity tokens are valid for an hour; refresh a few minutes early.
const TOKEN_REFRESH_MARGIN_MS = 5 * 60 * 1000;
let cachedToken = null;

/**
 * Fetches a Google-signed ID token for the Cloud Run service.
 *
 * Uses the metadata server directly rather than google-auth-library so the function keeps zero
 * runtime dependencies beyond the functions framework.
 */
async function getIdToken(audience) {
  if (SKIP_AUTH) return null;

  const now = Date.now();
  if (cachedToken && cachedToken.audience === audience && cachedToken.expiresAt > now) {
    return cachedToken.token;
  }

  const url = `${METADATA_IDENTITY_URL}?audience=${encodeURIComponent(audience)}`;
  const response = await fetch(url, { headers: { 'Metadata-Flavor': 'Google' } });
  if (!response.ok) {
    throw new Error(`Metadata server returned ${response.status} fetching an ID token`);
  }

  const token = (await response.text()).trim();
  cachedToken = { audience, token, expiresAt: now + 3600000 - TOKEN_REFRESH_MARGIN_MS };
  return token;
}

// =============================================================================
// CORS
// =============================================================================

function setCorsHeaders(res) {
  res.set('Access-Control-Allow-Origin', '*');
  res.set('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  res.set('Access-Control-Allow-Headers', 'Content-Type, Authorization, X-Correlation-ID');
  res.set('Access-Control-Max-Age', '3600');
}

// =============================================================================
// Execution Metadata Builder
//
// Only applied to responses this function generates itself (routing, contracts, and its own
// errors). Agent responses come back from the engine with the envelope already applied and are
// forwarded unmodified.
// =============================================================================

function buildExecutionMetadata(req) {
  return {
    trace_id: req.headers['x-correlation-id'] || crypto.randomUUID(),
    timestamp: new Date().toISOString(),
    service: SERVICE_NAME,
    execution_id: crypto.randomUUID(),
  };
}

function buildResponse(req, agentName, status, data, startTime) {
  const durationMs = Date.now() - startTime;
  const layers = [
    { layer: 'AGENT_ROUTING', status: 'completed' },
    { layer: `ORCHESTRATOR_${agentName.toUpperCase().replace(/-/g, '_')}`, status, duration_ms: durationMs },
  ];

  return {
    ...data,
    execution_metadata: buildExecutionMetadata(req),
    layers_executed: layers,
  };
}

// =============================================================================
// Engine proxy
// =============================================================================

/**
 * Forwards a request to the engine and returns its status and body unmodified.
 *
 * Fails closed: if the engine is unconfigured, unreachable, or times out, this returns a 5xx.
 * It never synthesises a success, which is the defect ADR-0001 exists to remove.
 */
async function proxyToEngine(req, path, body) {
  if (!ENGINE_URL) {
    return {
      statusCode: 503,
      data: {
        error: 'ORCHESTRATOR_ENGINE_URL is not configured; the orchestration engine is unreachable',
        engine_configured: false,
      },
    };
  }

  const correlationId = req.headers['x-correlation-id'] || crypto.randomUUID();
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), ENGINE_TIMEOUT_MS);

  try {
    const headers = {
      'Content-Type': 'application/json',
      'X-Correlation-ID': correlationId,
    };

    const token = await getIdToken(ENGINE_URL);
    if (token) headers.Authorization = `Bearer ${token}`;

    const response = await fetch(`${ENGINE_URL}${path}`, {
      method: body === undefined ? 'GET' : 'POST',
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: controller.signal,
    });

    const text = await response.text();
    let data;
    try {
      data = text ? JSON.parse(text) : {};
    } catch {
      // A non-JSON body from the engine is a real upstream fault, not something to paper over.
      return {
        statusCode: 502,
        data: {
          error: 'Engine returned a non-JSON response',
          upstream_status: response.status,
          upstream_body: text.slice(0, 2048),
        },
      };
    }

    return { statusCode: response.status, data, correlationId, passthrough: true };
  } catch (err) {
    const timedOut = err.name === 'AbortError';
    return {
      statusCode: timedOut ? 504 : 502,
      data: {
        error: timedOut
          ? `Engine did not respond within ${ENGINE_TIMEOUT_MS}ms`
          : `Engine request failed: ${err.message}`,
        engine_url: ENGINE_URL,
      },
    };
  } finally {
    clearTimeout(timer);
  }
}

// =============================================================================
// Health and readiness
//
// Both proxy the engine and fail closed. The previous implementation checked that seven
// constants were seven constants, so it could never report anything but healthy.
// =============================================================================

async function handleHealth(req) {
  const upstream = await proxyToEngine(req, '/health');
  const healthy = upstream.statusCode === 200;

  const agentStatuses = {};
  for (const agent of AGENTS) {
    agentStatuses[agent] = {
      status: healthy ? 'healthy' : 'unavailable',
      route: AGENT_ROUTES[agent],
      name: AGENT_DISPLAY_NAMES[agent],
    };
  }

  return {
    statusCode: healthy ? 200 : 503,
    data: {
      status: healthy ? 'healthy' : 'unhealthy',
      service: SERVICE_NAME,
      version: SERVICE_VERSION,
      engine: {
        url: ENGINE_URL || null,
        reachable: healthy,
        status_code: upstream.statusCode,
        detail: healthy ? undefined : upstream.data && upstream.data.error,
      },
      agents: agentStatuses,
      agents_list: AGENTS,
      base_url: BASE_URL,
      timestamp: new Date().toISOString(),
    },
  };
}

async function handleReady(req) {
  const upstream = await proxyToEngine(req, '/ready');
  const ready = upstream.statusCode === 200;

  return {
    statusCode: ready ? 200 : 503,
    data: {
      ready,
      service: SERVICE_NAME,
      version: SERVICE_VERSION,
      checks: {
        engine_configured: Boolean(ENGINE_URL),
        engine_reachable: ready,
        engine_status_code: upstream.statusCode,
      },
    },
  };
}

// =============================================================================
// Contract Endpoint (served locally -- it is static metadata, not orchestration)
// =============================================================================

function handleContracts(agentName) {
  if (agentName) {
    const contract = AGENT_CONTRACTS[agentName];
    if (!contract) {
      return { statusCode: 404, data: { error: `Unknown agent: ${agentName}` } };
    }
    return { statusCode: 200, data: contract };
  }
  return { statusCode: 200, data: AGENT_CONTRACTS };
}

// =============================================================================
// Router
// =============================================================================

function parseRoute(path) {
  const normalized = path.replace(/\/+$/, '') || '/';

  if (normalized === '/' || normalized === '') return { type: 'root' };
  if (normalized === '/health') return { type: 'health' };
  if (normalized === '/ready') return { type: 'ready' };

  // /v1/orchestrator/contracts/:agent?
  const contractMatch = normalized.match(/^\/v1\/orchestrator\/contracts(?:\/([a-z-]+))?$/);
  if (contractMatch) return { type: 'contracts', agent: contractMatch[1] || null };

  // /v1/orchestrator/:agent
  const agentMatch = normalized.match(/^\/v1\/orchestrator\/([a-z-]+)$/);
  if (agentMatch && AGENTS.includes(agentMatch[1])) {
    return { type: 'agent', agent: agentMatch[1] };
  }

  return { type: 'not_found' };
}

// =============================================================================
// Entry Point
// =============================================================================

/**
 * Cloud Function HTTP handler.
 *
 * @param {import('express').Request} req
 * @param {import('express').Response} res
 */
const handler = async (req, res) => {
  const startTime = Date.now();

  // CORS preflight
  setCorsHeaders(res);
  if (req.method === 'OPTIONS') {
    res.status(204).send('');
    return;
  }

  const route = parseRoute(req.path);

  switch (route.type) {
    // ------------------------------------------------------------------
    // Root
    // ------------------------------------------------------------------
    case 'root': {
      const body = buildResponse(req, 'ROUTER', 'completed', {
        service: SERVICE_NAME,
        version: SERVICE_VERSION,
        description: 'Orchestrator Agents Cloud Function — authenticating proxy to the Rust orchestration engine.',
        engine_url: ENGINE_URL || null,
        agents: AGENTS.map((a) => ({
          name: AGENT_DISPLAY_NAMES[a],
          route: AGENT_ROUTES[a],
          contract: `/v1/orchestrator/contracts/${a}`,
        })),
        endpoints: {
          health: '/health',
          ready: '/ready',
          contracts: '/v1/orchestrator/contracts',
        },
      }, startTime);
      res.status(200).json(body);
      return;
    }

    // ------------------------------------------------------------------
    // Health
    // ------------------------------------------------------------------
    case 'health': {
      const result = await handleHealth(req);
      const body = buildResponse(req, 'HEALTH', result.statusCode === 200 ? 'completed' : 'error', result.data, startTime);
      res.status(result.statusCode).json(body);
      return;
    }

    // ------------------------------------------------------------------
    // Ready
    // ------------------------------------------------------------------
    case 'ready': {
      const result = await handleReady(req);
      const body = buildResponse(req, 'READY', result.statusCode === 200 ? 'completed' : 'error', result.data, startTime);
      res.status(result.statusCode).json(body);
      return;
    }

    // ------------------------------------------------------------------
    // Contracts
    // ------------------------------------------------------------------
    case 'contracts': {
      const result = handleContracts(route.agent);
      const body = buildResponse(req, 'CONTRACTS', 'completed', result.data, startTime);
      res.status(result.statusCode).json(body);
      return;
    }

    // ------------------------------------------------------------------
    // Agent routes — proxied to the engine
    // ------------------------------------------------------------------
    case 'agent': {
      if (req.method !== 'POST') {
        const body = buildResponse(req, route.agent, 'error', {
          error: `Method ${req.method} not allowed. Use POST.`,
          agent: route.agent,
        }, startTime);
        res.status(405).json(body);
        return;
      }

      const result = await proxyToEngine(req, AGENT_ROUTES[route.agent], req.body || {});

      if (result.passthrough) {
        // The engine already applied the envelope. Forward status and body unmodified so there
        // is exactly one source of truth for what happened.
        res.set('X-Correlation-ID', result.correlationId);
        res.status(result.statusCode).json(result.data);
        return;
      }

      // Proxy-level failure: the engine never answered, so this function describes its own error.
      const body = buildResponse(req, route.agent, 'error', result.data, startTime);
      res.status(result.statusCode).json(body);
      return;
    }

    // ------------------------------------------------------------------
    // Not found
    // ------------------------------------------------------------------
    default: {
      const body = buildResponse(req, 'ROUTER', 'error', {
        error: 'Route not found',
        path: req.path,
        available_routes: Object.values(AGENT_ROUTES).concat(['/health', '/ready', '/v1/orchestrator/contracts']),
      }, startTime);
      res.status(404).json(body);
      return;
    }
  }
};

module.exports = { handler };
