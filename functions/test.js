'use strict';

// =============================================================================
// Tests for orchestrator-agents Cloud Function
// =============================================================================

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

function assertMetadata(res, testName) {
  const body = res._body;
  assert(body.execution_metadata !== undefined, `${testName}: has execution_metadata`);
  assert(body.execution_metadata.trace_id !== undefined, `${testName}: has trace_id`);
  assert(body.execution_metadata.timestamp !== undefined, `${testName}: has timestamp`);
  assert(body.execution_metadata.service === 'orchestrator-agents', `${testName}: service is orchestrator-agents`);
  assert(body.execution_metadata.execution_id !== undefined, `${testName}: has execution_id`);
  assert(Array.isArray(body.layers_executed), `${testName}: has layers_executed array`);
  assert(body.layers_executed.length >= 2, `${testName}: layers_executed has >= 2 entries`);
  assert(body.layers_executed[0].layer === 'AGENT_ROUTING', `${testName}: first layer is AGENT_ROUTING`);
}

// =============================================================================
// Test: Root endpoint
// =============================================================================
console.log('\n--- Root Endpoint ---');
{
  const req = mockReq('GET', '/');
  const res = mockRes();
  handler(req, res);
  assert(res._status === 200, 'Root returns 200');
  assert(res._body.service === 'orchestrator-agents', 'Root returns correct service');
  assert(res._body.version === '0.1.5', 'Root returns correct version');
  assert(Array.isArray(res._body.agents), 'Root lists agents');
  assert(res._body.agents.length === 7, 'Root lists 7 agents');
  assertMetadata(res, 'Root');
}

// =============================================================================
// Test: Health endpoint
// =============================================================================
console.log('\n--- Health Endpoint ---');
{
  const req = mockReq('GET', '/health');
  const res = mockRes();
  handler(req, res);
  assert(res._status === 200, 'Health returns 200');
  assert(res._body.status === 'healthy', 'Health status is healthy');
  assert(res._body.service === 'orchestrator-agents', 'Health service correct');
  const agents = res._body.agents_list;
  assert(Array.isArray(agents), 'Health agents_list is array');
  assert(agents.length === 7, 'Health lists 7 agents');
  assert(agents.includes('workflow'), 'Health includes workflow');
  assert(agents.includes('scheduler'), 'Health includes scheduler');
  assert(agents.includes('dependencies'), 'Health includes dependencies');
  assert(agents.includes('retry'), 'Health includes retry');
  assert(agents.includes('parallel'), 'Health includes parallel');
  assert(agents.includes('state-machine'), 'Health includes state-machine');
  assert(agents.includes('swarm'), 'Health includes swarm');
  assertMetadata(res, 'Health');
}

// =============================================================================
// Test: Ready endpoint
// =============================================================================
console.log('\n--- Ready Endpoint ---');
{
  const req = mockReq('GET', '/ready');
  const res = mockRes();
  handler(req, res);
  assert(res._status === 200, 'Ready returns 200');
  assert(res._body.ready === true, 'Ready is true');
  assert(res._body.checks.agents_registered === true, 'All agents registered');
  assert(res._body.checks.contracts_loaded === true, 'All contracts loaded');
  assert(res._body.checks.handlers_available === true, 'All handlers available');
  assertMetadata(res, 'Ready');
}

// =============================================================================
// Test: CORS preflight
// =============================================================================
console.log('\n--- CORS ---');
{
  const req = mockReq('OPTIONS', '/v1/orchestrator/workflow');
  const res = mockRes();
  handler(req, res);
  assert(res._status === 204, 'OPTIONS returns 204');
  assert(res._headers['Access-Control-Allow-Origin'] === '*', 'CORS origin header set');
  assert(res._headers['Access-Control-Allow-Methods'].includes('POST'), 'CORS methods include POST');
  assert(res._headers['Access-Control-Allow-Headers'].includes('X-Correlation-ID'), 'CORS headers include X-Correlation-ID');
}

// =============================================================================
// Test: X-Correlation-ID propagation
// =============================================================================
console.log('\n--- Correlation ID ---');
{
  const traceId = 'test-trace-12345';
  const req = mockReq('GET', '/health', {}, { 'x-correlation-id': traceId });
  const res = mockRes();
  handler(req, res);
  assert(res._body.execution_metadata.trace_id === traceId, 'trace_id matches X-Correlation-ID header');
}

// =============================================================================
// Test: Contracts endpoint
// =============================================================================
console.log('\n--- Contracts ---');
{
  const req = mockReq('GET', '/v1/orchestrator/contracts');
  const res = mockRes();
  handler(req, res);
  assert(res._status === 200, 'Contracts returns 200');
  assert(Object.keys(res._body).length > 7, 'Contracts returns all agents (+ metadata)');
  assertMetadata(res, 'Contracts');
}
{
  const req = mockReq('GET', '/v1/orchestrator/contracts/workflow');
  const res = mockRes();
  handler(req, res);
  assert(res._status === 200, 'Single contract returns 200');
  assert(res._body.agent_id === 'workflow-orchestrator', 'Workflow contract agent_id correct');
}

// =============================================================================
// Test: All 7 Agent Routes
// =============================================================================

const agentTests = [
  {
    name: 'Workflow Orchestrator',
    path: '/v1/orchestrator/workflow',
    body: { workflow_id: 'wf-1', workflow_name: 'test-workflow', tasks: [{ task_id: 't1', name: 'step1', task_type: 'transform' }] },
    agentId: 'workflow-orchestrator',
    layerPrefix: 'ORCHESTRATOR_WORKFLOW',
  },
  {
    name: 'Task Scheduler',
    path: '/v1/orchestrator/scheduler',
    body: { schedule_id: 'sch-1', tasks: [{ task_id: 't1', name: 'job1', schedule_type: 'immediate' }] },
    agentId: 'task-scheduler',
    layerPrefix: 'ORCHESTRATOR_SCHEDULER',
  },
  {
    name: 'Dependency Resolver',
    path: '/v1/orchestrator/dependencies',
    body: { request_id: 'req-1', workflow_id: 'wf-1', tasks: [{ task_id: 't1', name: 'step1' }] },
    agentId: 'dependency-resolver',
    layerPrefix: 'ORCHESTRATOR_DEPENDENCIES',
  },
  {
    name: 'Retry & Recovery',
    path: '/v1/orchestrator/retry',
    body: { request_id: 'req-1', failure: { task_id: 't1', error_category: 'transient' } },
    agentId: 'retry-recovery',
    layerPrefix: 'ORCHESTRATOR_RETRY',
  },
  {
    name: 'Parallelization',
    path: '/v1/orchestrator/parallel',
    body: { request_id: 'req-1', workflow_id: 'wf-1', tasks: [{ task_id: 't1', name: 'step1' }] },
    agentId: 'parallelization-agent',
    layerPrefix: 'ORCHESTRATOR_PARALLEL',
  },
  {
    name: 'State Machine',
    path: '/v1/orchestrator/state-machine',
    body: {
      request_id: 'req-1', execution_id: 'exec-1', entity_type: 'workflow',
      current_state: 'pending', target_state: 'running', reason: 'start execution', initiated_by: 'test',
    },
    agentId: 'state-machine-agent',
    layerPrefix: 'ORCHESTRATOR_STATE_MACHINE',
  },
  {
    name: 'Swarm Coordinator',
    path: '/v1/orchestrator/swarm',
    body: {
      request_id: 'req-1', workflow_id: 'wf-1',
      objective: { id: 'obj-1', description: 'test', objective_type: 'analysis' },
      workers: [{ worker_id: 'w1', agent_type: 'workflow_orchestrator', task: { task_id: 't1', description: 'test' } }],
    },
    agentId: 'swarm-coordinator-agent',
    layerPrefix: 'ORCHESTRATOR_SWARM',
  },
];

for (const test of agentTests) {
  console.log(`\n--- ${test.name} Agent ---`);

  // Valid POST
  {
    const req = mockReq('POST', test.path, test.body);
    const res = mockRes();
    handler(req, res);
    assert(res._status === 200, `${test.name}: valid POST returns 200`);
    assert(res._body.agent === test.agentId, `${test.name}: agent_id is ${test.agentId}`);
    assertMetadata(res, test.name);
    assert(res._body.layers_executed[1].layer === test.layerPrefix, `${test.name}: layer is ${test.layerPrefix}`);
    assert(res._body.layers_executed[1].status === 'completed', `${test.name}: layer status is completed`);
    assert(typeof res._body.layers_executed[1].duration_ms === 'number', `${test.name}: duration_ms is number`);
  }

  // Missing required fields
  {
    const req = mockReq('POST', test.path, {});
    const res = mockRes();
    handler(req, res);
    assert(res._status === 400, `${test.name}: empty body returns 400`);
    assert(res._body.error !== undefined, `${test.name}: error message present`);
    assertMetadata(res, `${test.name} (400)`);
  }

  // Wrong method
  {
    const req = mockReq('GET', test.path);
    const res = mockRes();
    handler(req, res);
    assert(res._status === 405, `${test.name}: GET returns 405`);
  }
}

// =============================================================================
// Test: State Machine validation logic
// =============================================================================
console.log('\n--- State Machine Transition Validation ---');
{
  const req = mockReq('POST', '/v1/orchestrator/state-machine', {
    request_id: 'req-1', execution_id: 'exec-1', entity_type: 'workflow',
    current_state: 'completed', target_state: 'running', reason: 'invalid', initiated_by: 'test',
  });
  const res = mockRes();
  handler(req, res);
  assert(res._status === 200, 'Invalid transition still returns 200');
  assert(res._body.success === false, 'Invalid transition reports success=false');
  assert(res._body.status === 'invalid', 'Invalid transition reports status=invalid');
  assert(res._body.new_state === 'completed', 'Invalid transition preserves current state');
}
{
  const req = mockReq('POST', '/v1/orchestrator/state-machine', {
    request_id: 'req-2', execution_id: 'exec-2', entity_type: 'workflow',
    current_state: 'completed', target_state: 'running', reason: 'forced', initiated_by: 'admin',
    force: true,
  });
  const res = mockRes();
  handler(req, res);
  assert(res._body.success === true, 'Forced transition reports success=true');
  assert(res._body.new_state === 'running', 'Forced transition changes state');
}

// =============================================================================
// Test: 404
// =============================================================================
console.log('\n--- 404 ---');
{
  const req = mockReq('GET', '/v1/orchestrator/nonexistent');
  const res = mockRes();
  handler(req, res);
  assert(res._status === 404, 'Unknown route returns 404');
  assert(Array.isArray(res._body.available_routes), '404 includes available_routes');
  assertMetadata(res, '404');
}

// =============================================================================
// Test: Contract schemas completeness
// =============================================================================
console.log('\n--- Contract Schemas ---');
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

// =============================================================================
// Summary
// =============================================================================
console.log('\n========================================');
console.log(`Results: ${passed} passed, ${failed} failed, ${passed + failed} total`);
console.log('========================================\n');

if (failed > 0) {
  process.exit(1);
}
