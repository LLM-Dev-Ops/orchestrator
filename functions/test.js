'use strict';

// =============================================================================
// Tests for orchestrator-agents Cloud Function
//
// The function is a proxy, so these assert proxy behaviour: that requests reach the engine with
// the right method, path and correlation ID; that the engine's status and body come back
// unmodified; and that every failure mode fails closed. They deliberately do NOT assert envelope
// shape on agent routes -- the previous suite asserted only that, and passed against a service
// that orchestrated nothing.
//
// Orchestration correctness is asserted where the orchestration now lives, in
// crates/llm-orchestrator-cli/src/orchestrator_routes.rs: diamond-DAG ordering, cycle rejection,
// and dangling-dependency rejection. See ADR-0001.
// =============================================================================

const assertLib = require('assert');

process.env.ORCHESTRATOR_ENGINE_URL = 'https://engine.test.invalid';
process.env.ORCHESTRATOR_ENGINE_SKIP_AUTH = 'true';

const { handler } = require('./index');
const { AGENT_CONTRACTS } = require('./contracts');

let passed = 0;
let failed = 0;

function assert(condition, message) {
  if (!condition) {
    console.error(`  FAIL: ${message}`);
    failed++;
  } else {
    console.log(`  PASS: ${message}`);
    passed++;
  }
}

function mockReq(method, path, body, headers) {
  return {
    method: method || 'GET',
    path: path || '/',
    body: body || {},
    headers: headers || {},
  };
}

function mockRes() {
  const res = {
    _status: null,
    _body: null,
    _headers: {},
    status(code) { res._status = code; return res; },
    json(body) { res._body = body; return res; },
    send(body) { res._body = body; return res; },
    set(key, value) { res._headers[key] = value; return res; },
  };
  return res;
}

// ---------------------------------------------------------------------------
// fetch stub -- records every call so we can assert what reached the engine
// ---------------------------------------------------------------------------

const realFetch = global.fetch;
let fetchCalls = [];

function stubFetch(responder) {
  fetchCalls = [];
  global.fetch = async (url, init) => {
    fetchCalls.push({ url, init });
    return responder(url, init);
  };
}

function engineResponse(status, body) {
  return {
    ok: status >= 200 && status < 300,
    status,
    text: async () => (typeof body === 'string' ? body : JSON.stringify(body)),
  };
}

function restoreFetch() {
  global.fetch = realFetch;
}

function assertMetadata(res, testName) {
  const body = res._body;
  assert(body.execution_metadata !== undefined, `${testName}: has execution_metadata`);
  assert(body.execution_metadata.service === 'orchestrator-agents', `${testName}: service is orchestrator-agents`);
  assert(Array.isArray(body.layers_executed), `${testName}: has layers_executed array`);
  assert(body.layers_executed[0].layer === 'AGENT_ROUTING', `${testName}: first layer is AGENT_ROUTING`);
}

async function run() {
  // =========================================================================
  console.log('\n--- Agent routes reach the engine ---');
  // =========================================================================
  {
    stubFetch(() => engineResponse(200, { status: 'resolved', execution_order: ['A', 'B'] }));

    const req = mockReq('POST', '/v1/orchestrator/dependencies', {
      request_id: 'r-1',
      workflow_id: 'wf-1',
      tasks: [{ task_id: 'A', name: 'ingest' }],
    }, { 'x-correlation-id': 'trace-123' });
    const res = mockRes();
    await handler(req, res);

    assert(fetchCalls.length === 1, 'Engine was called exactly once');
    const call = fetchCalls[0];
    assert(
      call.url === 'https://engine.test.invalid/v1/orchestrator/dependencies',
      `Engine URL is the dependencies route (got ${call.url})`,
    );
    assert(call.init.method === 'POST', 'Engine is called with POST');
    assert(
      call.init.headers['X-Correlation-ID'] === 'trace-123',
      'Caller correlation ID is propagated upstream',
    );
    assert(
      JSON.parse(call.init.body).tasks[0].task_id === 'A',
      'Request body is forwarded to the engine',
    );
    restoreFetch();
  }

  // =========================================================================
  console.log('\n--- Engine response is forwarded unmodified ---');
  // =========================================================================
  {
    const engineBody = {
      status: 'resolved',
      execution_order: ['A', 'B', 'C', 'D'],
      execution_metadata: { trace_id: 'from-engine', service: 'orchestrator-agents' },
      layers_executed: [{ layer: 'AGENT_ROUTING', status: 'completed' }],
    };
    stubFetch(() => engineResponse(200, engineBody));

    const res = mockRes();
    await handler(mockReq('POST', '/v1/orchestrator/dependencies', { request_id: 'r', workflow_id: 'w', tasks: [] }), res);

    assert(res._status === 200, 'Upstream 200 is propagated');
    assertLib.deepStrictEqual(res._body, engineBody);
    assert(true, 'Body is identical to the engine response -- the proxy adds nothing');
    assert(
      res._body.execution_metadata.trace_id === 'from-engine',
      'The engine owns the envelope; the proxy does not overwrite it',
    );
    restoreFetch();
  }

  // =========================================================================
  console.log('\n--- Engine error statuses are propagated, not swallowed ---');
  // =========================================================================
  {
    stubFetch(() => engineResponse(400, { error: 'Cyclic dependency detected' }));

    const res = mockRes();
    await handler(mockReq('POST', '/v1/orchestrator/dependencies', { request_id: 'r', workflow_id: 'w', tasks: [] }), res);

    assert(res._status === 400, 'A cyclic graph comes back as 400, not 200');
    assert(res._body.error === 'Cyclic dependency detected', 'The engine error message reaches the caller');
    restoreFetch();
  }

  // =========================================================================
  console.log('\n--- All seven agents route to their own engine path ---');
  // =========================================================================
  {
    const expected = {
      workflow: '/v1/orchestrator/workflow',
      scheduler: '/v1/orchestrator/scheduler',
      dependencies: '/v1/orchestrator/dependencies',
      retry: '/v1/orchestrator/retry',
      parallel: '/v1/orchestrator/parallel',
      'state-machine': '/v1/orchestrator/state-machine',
      swarm: '/v1/orchestrator/swarm',
    };

    for (const [agent, path] of Object.entries(expected)) {
      stubFetch(() => engineResponse(200, { ok: true }));
      const res = mockRes();
      await handler(mockReq('POST', `/v1/orchestrator/${agent}`, {}), res);
      assert(
        fetchCalls.length === 1 && fetchCalls[0].url === `https://engine.test.invalid${path}`,
        `${agent} proxies to ${path}`,
      );
      restoreFetch();
    }
  }

  // =========================================================================
  console.log('\n--- Fails closed when the engine is unreachable ---');
  // =========================================================================
  {
    stubFetch(() => { throw new Error('ECONNREFUSED'); });

    const res = mockRes();
    await handler(mockReq('POST', '/v1/orchestrator/dependencies', { request_id: 'r', workflow_id: 'w', tasks: [] }), res);

    assert(res._status === 502, 'An unreachable engine yields 502, never a fabricated 200');
    assert(/ECONNREFUSED/.test(res._body.error), 'The transport failure is reported');
    restoreFetch();
  }

  // =========================================================================
  console.log('\n--- Fails closed on timeout ---');
  // =========================================================================
  {
    stubFetch(() => {
      const err = new Error('aborted');
      err.name = 'AbortError';
      throw err;
    });

    const res = mockRes();
    await handler(mockReq('POST', '/v1/orchestrator/parallel', {}), res);

    assert(res._status === 504, 'A timeout yields 504');
    restoreFetch();
  }

  // =========================================================================
  console.log('\n--- Fails closed on a non-JSON engine response ---');
  // =========================================================================
  {
    stubFetch(() => engineResponse(200, '<html>502 Bad Gateway</html>'));

    const res = mockRes();
    await handler(mockReq('POST', '/v1/orchestrator/swarm', {}), res);

    assert(res._status === 502, 'A non-JSON upstream body yields 502');
    restoreFetch();
  }

  // =========================================================================
  console.log('\n--- Readiness fails closed (ADR-0001 Test 5) ---');
  // =========================================================================
  {
    stubFetch(() => { throw new Error('ENOTFOUND'); });

    const res = mockRes();
    await handler(mockReq('GET', '/ready'), res);

    assert(res._status !== 200, `/ready is non-200 when the engine is unreachable (got ${res._status})`);
    assert(res._body.ready === false, '/ready reports ready: false');
    assert(res._body.checks.engine_reachable === false, '/ready reports the engine unreachable');
    restoreFetch();
  }

  {
    stubFetch(() => engineResponse(200, { ready: true }));

    const res = mockRes();
    await handler(mockReq('GET', '/ready'), res);

    assert(res._status === 200, '/ready is 200 when the engine answers');
    assert(res._body.ready === true, '/ready reports ready: true');
    restoreFetch();
  }

  // =========================================================================
  console.log('\n--- Health reflects the engine, not a constant ---');
  // =========================================================================
  {
    stubFetch(() => { throw new Error('ECONNREFUSED'); });

    const res = mockRes();
    await handler(mockReq('GET', '/health'), res);

    assert(res._status === 503, '/health is 503 when the engine is down');
    assert(res._body.status === 'unhealthy', '/health reports unhealthy');
    assert(
      res._body.agents.workflow.status === 'unavailable',
      'Per-agent status follows the engine rather than being hardcoded healthy',
    );
    restoreFetch();
  }

  {
    stubFetch(() => engineResponse(200, { status: 'healthy' }));

    const res = mockRes();
    await handler(mockReq('GET', '/health', {}, { 'x-correlation-id': 'trace-health' }), res);

    assert(res._status === 200, '/health is 200 when the engine answers');
    assert(res._body.status === 'healthy', '/health reports healthy');
    assert(res._body.agents_list.length === 7, '/health lists 7 agents');
    assert(res._body.execution_metadata.trace_id === 'trace-health', 'trace_id matches X-Correlation-ID');
    assertMetadata(res, 'Health');
    restoreFetch();
  }

  // =========================================================================
  console.log('\n--- The function no longer orchestrates locally ---');
  // =========================================================================
  {
    // The old handleStateMachine validated transitions itself against the contract table.
    // If any local orchestration survived, the engine would not be called.
    stubFetch(() => engineResponse(200, { status: 'completed' }));

    const res = mockRes();
    await handler(mockReq('POST', '/v1/orchestrator/state-machine', {
      request_id: 'r', execution_id: 'e', entity_type: 'task',
      current_state: 'completed', target_state: 'running',
      reason: 'test', initiated_by: 'test',
    }), res);

    assert(fetchCalls.length === 1, 'state-machine defers to the engine instead of deciding locally');
    restoreFetch();

    const source = require('fs').readFileSync(require.resolve('./index.js'), 'utf8');
    assert(
      !/contract\.state_machine\[/.test(source),
      'The local transition-table lookup has been deleted -- one source of truth',
    );
  }

  // =========================================================================
  console.log('\n--- Unconfigured engine fails closed ---');
  // =========================================================================
  {
    delete require.cache[require.resolve('./index.js')];
    const savedUrl = process.env.ORCHESTRATOR_ENGINE_URL;
    delete process.env.ORCHESTRATOR_ENGINE_URL;
    const { handler: unconfigured } = require('./index.js');

    const res = mockRes();
    await unconfigured(mockReq('POST', '/v1/orchestrator/workflow', {}), res);
    assert(res._status === 503, 'An unconfigured engine yields 503');
    assert(res._body.engine_configured === false, 'The response says the engine is unconfigured');

    const readyRes = mockRes();
    await unconfigured(mockReq('GET', '/ready'), readyRes);
    assert(readyRes._status === 503, '/ready is 503 with no engine configured');

    process.env.ORCHESTRATOR_ENGINE_URL = savedUrl;
    delete require.cache[require.resolve('./index.js')];
  }

  // =========================================================================
  console.log('\n--- Routing, CORS and contracts stay local ---');
  // =========================================================================
  {
    const res = mockRes();
    await handler(mockReq('OPTIONS', '/v1/orchestrator/workflow'), res);
    assert(res._status === 204, 'CORS preflight returns 204');
    assert(res._headers['Access-Control-Allow-Origin'] === '*', 'CORS origin header set');
    assert(res._headers['Access-Control-Allow-Methods'].includes('POST'), 'CORS methods include POST');
    assert(res._headers['Access-Control-Allow-Headers'].includes('X-Correlation-ID'), 'CORS headers include X-Correlation-ID');
  }

  {
    const res = mockRes();
    await handler(mockReq('GET', '/'), res);
    assert(res._status === 200, 'Root returns 200');
    assert(res._body.agents.length === 7, 'Root lists 7 agents');
    assertMetadata(res, 'Root');
  }

  {
    const res = mockRes();
    await handler(mockReq('GET', '/v1/orchestrator/contracts'), res);
    assert(res._status === 200, 'Contracts are served without calling the engine');
    assert(Object.keys(res._body).length >= 7, 'All contracts returned');
  }

  {
    const res = mockRes();
    await handler(mockReq('GET', '/v1/orchestrator/contracts/dependencies'), res);
    assert(res._status === 200, 'Single contract returns 200');
    assert(res._body.agent_id === 'dependency-resolver', 'Correct contract returned');
  }

  {
    const res = mockRes();
    await handler(mockReq('GET', '/v1/orchestrator/contracts/nope'), res);
    assert(res._status === 404, 'Unknown contract returns 404');
  }

  {
    const res = mockRes();
    await handler(mockReq('GET', '/v1/orchestrator/dependencies'), res);
    assert(res._status === 405, 'GET on an agent route returns 405');
  }

  {
    const res = mockRes();
    await handler(mockReq('GET', '/v1/orchestrator/nonexistent'), res);
    assert(res._status === 404, 'Unknown route returns 404');
    assert(Array.isArray(res._body.available_routes), '404 includes available_routes');
    assertMetadata(res, '404');
  }

  // =========================================================================
  console.log('\n--- Contract schemas ---');
  // =========================================================================
  {
    const expectedAgents = ['workflow', 'scheduler', 'dependencies', 'retry', 'parallel', 'state-machine', 'swarm'];
    for (const agent of expectedAgents) {
      const contract = AGENT_CONTRACTS[agent];
      assert(contract !== undefined, `Contract exists for ${agent}`);
      assert(contract.agent_id !== undefined, `${agent} contract has agent_id`);
      assert(contract.agent_version !== undefined, `${agent} contract has agent_version`);
      assert(contract.classification !== undefined, `${agent} contract has classification`);
      assert(contract.description !== undefined, `${agent} contract has description`);
      assert(contract.input !== undefined, `${agent} contract has input schema`);
      assert(contract.output !== undefined, `${agent} contract has output schema`);
    }
  }

  console.log('\n========================================');
  console.log(`Results: ${passed} passed, ${failed} failed, ${passed + failed} total`);
  console.log('========================================\n');

  if (failed > 0) {
    process.exit(1);
  }
}

run().catch((err) => {
  console.error(err);
  process.exit(1);
});
