const http = require('node:http');
const {resolve} = require('node:path');
const assert = require('node:assert/strict');
const LIMIT = 1048576;
const PRIVATE = 'private-upstream-marker-DO-NOT-ECHO';

(async () => {
  const cases = {
    html: {status: 200, type: 'text/html', text: `<html>${PRIVATE}</html>`},
    malformed: {status: 500, type: 'application/json', text: `{${PRIVATE}`},
    large_json: {status: 200, type: 'application/json', text: JSON.stringify({value: 'x'.repeat(LIMIT)})},
    unicode_bytes: {status: 200, type: 'application/json', text: JSON.stringify({value: 'é'.repeat(LIMIT / 2)})},
    chunked: {status: 200, type: 'application/json', text: JSON.stringify({value: 'x'.repeat(LIMIT)}), chunked: true},
    large_html: {status: 503, type: 'text/html', text: PRIVATE + 'x'.repeat(LIMIT)},
  };
  for (const status of [200, 401, 429, 503]) {
    cases[`anchor_${status}`] = {status, type: 'application/json', text: JSON.stringify({status, content: 'unchanged', execution_metadata: {trace_id: 'engine-trace'}})};
  }
  cases.near_limit = {status: 200, type: 'application/json', text: JSON.stringify({value: 'x'.repeat(LIMIT - 100)})};
  const upstreamCalls = [];
  const server = http.createServer(async (req, res) => {
    let body = '';
    for await (const chunk of req) body += chunk;
    const id = JSON.parse(body).case;
    upstreamCalls.push({id, path: req.url, correlation: req.headers['x-correlation-id']});
    const item = cases[id];
    res.writeHead(item.status, {'Content-Type': item.type});
    if (item.chunked) {
      res.write(item.text.slice(0, 65536));
      setImmediate(() => res.end(item.text.slice(65536)));
    } else res.end(item.text);
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  process.env.ORCHESTRATOR_ENGINE_URL = `http://127.0.0.1:${server.address().port}`;
  process.env.ORCHESTRATOR_ENGINE_SKIP_AUTH = 'true';
  process.env.ORCHESTRATOR_ENGINE_TIMEOUT_MS = '3000';
  const {handler} = require(resolve(process.argv[2] || '.', 'functions/index.js'));
  const report = {holdout: [], anchors: []};
  try {
    for (const [id, item] of Object.entries(cases)) {
      const headers = {};
      let status, data;
      const req = {method: 'POST', path: '/v1/orchestrator/workflow', headers: {'x-correlation-id': 'native-probe'}, body: {case: id}};
      const res = {set(k, v) {headers[k] = v; return this;}, status(v) {status = v; return this;}, json(v) {data = v; return this;}, send(v) {data = v; return this;}};
      await handler(req, res);
      const call = upstreamCalls.at(-1);
      const preserved = call.id === id && call.path === req.path && call.correlation === 'native-probe' && headers['Access-Control-Allow-Origin'] === '*';
      const anchor = id.startsWith('anchor_') || id === 'near_limit';
      if (anchor) {
        report.anchors.push({name: id, passed: preserved && status === item.status && JSON.stringify(data) === item.text && headers['X-Correlation-ID'] === 'native-probe'});
      } else {
        const sanitized = !JSON.stringify(data).includes(PRIVATE) && !Object.hasOwn(data, 'upstream_body');
        const bounded = id === 'html' || id === 'malformed' || /exceeds/.test(data.error);
        report.holdout.push({name: id, status, passed: preserved && status === 502 && sanitized && bounded && data.layers_executed?.[1]?.status === 'error'});
      }
    }
  } finally {
    server.closeAllConnections();
    await new Promise(resolve => server.close(resolve));
  }
  assert.equal(report.holdout.length, 6);
  assert.equal(report.anchors.length, 5);
  console.log(JSON.stringify(report));
  process.exitCode = report.holdout.every(x => x.passed) && report.anchors.every(x => x.passed) ? 0 : 1;
})().catch(error => {console.error(error); process.exitCode = 1;});
