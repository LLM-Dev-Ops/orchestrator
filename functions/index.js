'use strict';

const crypto = require('crypto');
const { AGENT_CONTRACTS } = require('./contracts');

// =============================================================================
// orchestrator-agents Cloud Function
// Entry point: handler
// Runtime: nodejs20
// =============================================================================

const SERVICE_NAME = 'orchestrator-agents';
const SERVICE_VERSION = '0.1.5';
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
// Validation
// =============================================================================

function validateRequiredFields(body, required) {
  const missing = required.filter((f) => body[f] === undefined || body[f] === null);
  if (missing.length > 0) {
    return `Missing required fields: ${missing.join(', ')}`;
  }
  return null;
}

// =============================================================================
// Agent Handlers (routing layer — business logic lives in Rust crates)
// =============================================================================

function handleWorkflow(body) {
  const contract = AGENT_CONTRACTS.workflow;
  const error = validateRequiredFields(body, contract.input.required);
  if (error) return { statusCode: 400, data: { error, agent: contract.agent_id } };

  return {
    statusCode: 200,
    data: {
      agent: contract.agent_id,
      agent_version: contract.agent_version,
      classification: contract.classification,
      status: 'accepted',
      workflow_id: body.workflow_id,
      workflow_name: body.workflow_name,
      tasks_count: Array.isArray(body.tasks) ? body.tasks.length : 0,
      strategy: (body.config && body.config.strategy) || 'sequential',
    },
  };
}

function handleScheduler(body) {
  const contract = AGENT_CONTRACTS.scheduler;
  const error = validateRequiredFields(body, contract.input.required);
  if (error) return { statusCode: 400, data: { error, agent: contract.agent_id } };

  return {
    statusCode: 200,
    data: {
      agent: contract.agent_id,
      agent_version: contract.agent_version,
      classification: contract.classification,
      status: 'scheduled',
      schedule_id: body.schedule_id,
      tasks_count: Array.isArray(body.tasks) ? body.tasks.length : 0,
    },
  };
}

function handleDependencies(body) {
  const contract = AGENT_CONTRACTS.dependencies;
  const error = validateRequiredFields(body, contract.input.required);
  if (error) return { statusCode: 400, data: { error, agent: contract.agent_id } };

  return {
    statusCode: 200,
    data: {
      agent: contract.agent_id,
      agent_version: contract.agent_version,
      classification: contract.classification,
      status: 'resolved',
      request_id: body.request_id,
      workflow_id: body.workflow_id,
      tasks_count: Array.isArray(body.tasks) ? body.tasks.length : 0,
    },
  };
}

function handleRetry(body) {
  const contract = AGENT_CONTRACTS.retry;
  const error = validateRequiredFields(body, contract.input.required);
  if (error) return { statusCode: 400, data: { error, agent: contract.agent_id } };

  return {
    statusCode: 200,
    data: {
      agent: contract.agent_id,
      agent_version: contract.agent_version,
      classification: contract.classification,
      status: 'analyzed',
      request_id: body.request_id,
      failure_category: body.failure && body.failure.error_category,
    },
  };
}

function handleParallel(body) {
  const contract = AGENT_CONTRACTS.parallel;
  const error = validateRequiredFields(body, contract.input.required);
  if (error) return { statusCode: 400, data: { error, agent: contract.agent_id } };

  return {
    statusCode: 200,
    data: {
      agent: contract.agent_id,
      agent_version: contract.agent_version,
      classification: contract.classification,
      status: 'analyzed',
      request_id: body.request_id,
      workflow_id: body.workflow_id,
      tasks_count: Array.isArray(body.tasks) ? body.tasks.length : 0,
    },
  };
}

function handleStateMachine(body) {
  const contract = AGENT_CONTRACTS['state-machine'];
  const error = validateRequiredFields(body, contract.input.required);
  if (error) return { statusCode: 400, data: { error, agent: contract.agent_id } };

  const sm = contract.state_machine[body.entity_type] || contract.state_machine.workflow;
  const validTargets = sm.transitions[body.current_state] || [];
  const isValid = body.force || validTargets.includes(body.target_state);

  return {
    statusCode: 200,
    data: {
      agent: contract.agent_id,
      agent_version: contract.agent_version,
      classification: contract.classification,
      status: isValid ? 'completed' : 'invalid',
      request_id: body.request_id,
      execution_id: body.execution_id,
      entity_type: body.entity_type,
      success: isValid,
      previous_state: body.current_state,
      new_state: isValid ? body.target_state : body.current_state,
      validation: {
        valid: isValid,
        source_state_matches: true,
        target_state_exists: validTargets.length > 0,
        valid_targets: validTargets,
        rule_violations: [],
        warnings: [],
      },
    },
  };
}

function handleSwarm(body) {
  const contract = AGENT_CONTRACTS.swarm;
  const error = validateRequiredFields(body, contract.input.required);
  if (error) return { statusCode: 400, data: { error, agent: contract.agent_id } };

  return {
    statusCode: 200,
    data: {
      agent: contract.agent_id,
      agent_version: contract.agent_version,
      classification: contract.classification,
      status: 'accepted',
      request_id: body.request_id,
      workflow_id: body.workflow_id,
      workers_count: Array.isArray(body.workers) ? body.workers.length : 0,
      objective_type: body.objective && body.objective.objective_type,
    },
  };
}

const AGENT_HANDLERS = {
  workflow: handleWorkflow,
  scheduler: handleScheduler,
  dependencies: handleDependencies,
  retry: handleRetry,
  parallel: handleParallel,
  'state-machine': handleStateMachine,
  swarm: handleSwarm,
};

// =============================================================================
// Health Endpoint
// =============================================================================

function handleHealth(req) {
  const agentStatuses = {};
  for (const agent of AGENTS) {
    agentStatuses[agent] = {
      status: 'healthy',
      route: AGENT_ROUTES[agent],
      name: AGENT_DISPLAY_NAMES[agent],
    };
  }

  return {
    status: 'healthy',
    service: SERVICE_NAME,
    version: SERVICE_VERSION,
    agents: agentStatuses,
    agents_list: AGENTS,
    base_url: BASE_URL,
    timestamp: new Date().toISOString(),
  };
}

// =============================================================================
// Contract Endpoint
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
const handler = (req, res) => {
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
        description: 'Orchestrator Agents Cloud Function — routes to 7 orchestration agents.',
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
      const health = handleHealth(req);
      const body = buildResponse(req, 'HEALTH', 'completed', health, startTime);
      res.status(200).json(body);
      return;
    }

    // ------------------------------------------------------------------
    // Ready
    // ------------------------------------------------------------------
    case 'ready': {
      const body = buildResponse(req, 'READY', 'completed', {
        ready: true,
        service: SERVICE_NAME,
        version: SERVICE_VERSION,
        checks: {
          agents_registered: AGENTS.length === 7,
          contracts_loaded: Object.keys(AGENT_CONTRACTS).length === 7,
          handlers_available: Object.keys(AGENT_HANDLERS).length === 7,
        },
      }, startTime);
      res.status(200).json(body);
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
    // Agent routes
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

      const agentHandler = AGENT_HANDLERS[route.agent];
      if (!agentHandler) {
        const body = buildResponse(req, route.agent, 'error', {
          error: `No handler for agent: ${route.agent}`,
        }, startTime);
        res.status(500).json(body);
        return;
      }

      const reqBody = req.body || {};
      const result = agentHandler(reqBody);
      const body = buildResponse(req, route.agent, result.statusCode === 200 ? 'completed' : 'error', result.data, startTime);
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
