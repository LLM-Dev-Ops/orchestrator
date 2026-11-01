# Proxy response boundary

The Cloud Function continues to forward valid engine JSON and HTTP statuses unchanged.
Upstream response bodies are limited to 1,048,576 raw bytes, including multibyte UTF-8
characters. Responses exceeding this limit are cancelled and return a local 502 error.
Callers requiring larger engine results must split the result or arrange a separate
reviewed transfer contract; the proxy does not silently truncate successful JSON.

Malformed/non-JSON engine responses return 502 with the upstream status, but never
include upstream body text. Existing request timeouts, metadata-server authentication,
Rust-owned orchestration and CORS behavior are retained. This does not sanitize valid
JSON application errors or change deployment/IAM configuration.

Run `node functions/test.js` for existing assertions and
`node tests/proxy-response-probe.cjs .` for real loopback upstream regression checks.
The latter invokes the exported handler with a request/response adapter; it is not
a deployed Cloud Functions or metadata-server authentication test.
