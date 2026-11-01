const fs = require('node:fs');
const {spawnSync, execFileSync} = require('node:child_process');
const {resolve, join} = require('node:path');
const {createHash} = require('node:crypto');
const [base, head, tools, out] = process.argv.slice(2).map(x => resolve(x));
const hash = x => createHash('sha256').update(x).digest('hex');
for (const p of ['tests/proxy-response-probe.cjs', 'functions/test.js', 'Cargo.lock']) {
  if (!fs.readFileSync(join(base, p)).equals(fs.readFileSync(join(head, p)))) throw Error('Frozen bytes changed: ' + p);
}
fs.mkdirSync(out + '.logs', {recursive: true});
function run(root, label, key, command, args, seconds = 60) {
  // GNU timeout owns the subprocess group, so compiler descendants are bounded too.
  const started = Date.now();
  const r = spawnSync('timeout', ['--kill-after=5', String(seconds), command, ...args], {
    cwd: root, encoding: 'utf8', timeout: (seconds + 10) * 1000, maxBuffer: 32 * 1024 * 1024,
    env: {...process.env, CARGO_TARGET_DIR: '/tmp/orchestrator-oct31e-target', CARGO_PROFILE_DEV_DEBUG: '0', CARGO_PROFILE_TEST_DEBUG: '0'},
  });
  const text = (r.stdout || '') + (r.stderr || '');
  fs.writeFileSync(join(out + '.logs', label + '-' + key + '.log'), text);
  return {command: ['timeout', '--kill-after=5', String(seconds), command, ...args], exit: r.status, signal: r.signal,
    error: r.error?.message, ms: Date.now() - started, sha256: hash(text), pass: r.status === 0 && !r.error && !r.signal};
}
function measure(root, label) {
  const checks = {
    syntax: run(root, label, 'syntax', process.execPath, ['--check', 'functions/index.js'], 15),
    probe: run(root, label, 'probe', process.execPath, ['tests/proxy-response-probe.cjs', '.'], 20),
    node: run(root, label, 'node', process.execPath, ['functions/test.js'], 30),
  };
  const probe = JSON.parse(fs.readFileSync(join(out + '.logs', label + '-probe.log')));
  const passingAssertions = fs.readFileSync(join(out + '.logs', label + '-node.log'), 'utf8').split('\n').filter(x => x.startsWith('  PASS: ')).sort();
  if (passingAssertions.length !== 113) checks.node.pass = false;
  for (const op of ['test', 'build', 'clippy']) checks[op] = run(root, label, op, 'cargo',
    ['+1.88.0', op, '--workspace', '--all-features', '--locked', '--offline', '-j2', ...(op === 'clippy' ? ['--all-targets', '--', '-D', 'warnings'] : [])]);
  checks.fmt = run(root, label, 'fmt', 'cargo', ['+1.88.0', 'fmt', '--all', '--', '--check'], 30);
  checks.audit = run(root, label, 'audit', 'cargo', ['audit', '--file', 'Cargo.lock', '--no-fetch', '--json'], 30);
  checks.npmAudit = run(root, label, 'npm-audit', 'npm', ['audit', '--json'], 30);
  return {checks, probe, passingAssertions};
}
(async () => {
  const baseline = measure(base, 'baseline'), candidate = measure(head, 'candidate');
  const lib = join(tools, 'node_modules/@metaharness/flywheel');
  if (require(join(lib, 'package.json')).version !== '0.1.12') throw Error('Version drift');
  const fw = await import(join(lib, 'dist/index.js'));
  const revision = root => execFileSync('git', ['rev-parse', 'HEAD'], {cwd: root, encoding: 'utf8'}).trim();
  const b = revision(base), h = revision(head), path = 'functions/index.js', parentBytes = fs.readFileSync(join(base, path), 'utf8');
  const lostPassingAssertions = baseline.passingAssertions.filter(x => !candidate.passingAssertions.includes(x));
  function score(m, suite) {
    const items = m.probe[suite === 'holdout' ? 'holdout' : 'anchors'];
    const primary = items.filter(x => x.passed).length / items.length;
    return {primary, noopRate: 1 - primary, costPerWin: 0, regressed: Object.values(m.checks).some(x => !x.pass) || lostPassingAssertions.length > 0};
  }
  const result = await fw.runFlywheelGenerations({rootPolicy: {revision: b}, proposer: async () => ({value: h,
    summary: 'Bound upstream responses to1MiB and redact malformed response text', inverse: {path, parentBytes, sha256: hash(parentBytes)}}),
    evaluator: async (p, s) => score(p.revision === b ? baseline : candidate, s.id),
    holdout: {id: 'holdout', items: candidate.probe.holdout.map(x => x.name)}, anchor: {id: 'anchor', items: candidate.probe.anchors.map(x => x.name)},
    mutationTargets: ['revision'], maxGenerations: 1, signer: fw.makeSigner(), promotionRule: fw.meetsPromotionRule, dataSource: 'LIVE'});
  const fingerprint = fw.gateFingerprint(fw.meetsPromotionRule), replay = fw.verifyReplayBundle(result.replayBundle, {pinnedGateFingerprint: fingerprint});
  fs.writeFileSync(out, JSON.stringify({baselineRevision: b, candidateRevision: h, baseline, candidate, lostPassingAssertions, result, replay, fingerprint,
    rollback: {path, parentBytes, sha256: hash(parentBytes)}, limitations: 'Original113assertions use existing fetch stubs; new probe uses real loopback HTTP with handler adapter, not deployed IAM/framework integration. Neutral unmeasured cost; noopRate is failed-case fraction. Rust sources/lock unchanged; all broad check failures reject promotion.'}, null, 2));
  console.log(JSON.stringify({promotions: result.promotions.length, replay, lostPassingAssertions}));
})().catch(error => {console.error(error); process.exitCode = 1;});
