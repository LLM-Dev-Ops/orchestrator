# OpenTelemetry Tracing Guide

## Overview
Using distributed tracing to debug performance issues and understand request flow.

## Accessing Traces

Jaeger UI: `https://jaeger.example.com`

## Common Trace Analysis

### Slow Request Investigation

1. Open Jaeger UI
2. Select "orchestrator" service
3. Search for traces with duration > 5s
4. Open slow trace
5. Analyze span timeline:
   - Database queries
   - LLM API calls
   - Internal processing
6. Identify bottleneck

### Trace a Specific Workflow

```bash
# Find trace ID in logs
kubectl logs -n llm-orchestrator -l app=orchestrator | \
  grep "workflow_id=abc123" | grep "trace_id"

# Search Jaeger for trace_id
```

### Analyze Error

1. Search Jaeger for error traces
2. Look for spans with error tags
3. Check span logs for error details
4. Trace error propagation through system

## Trace Attributes

Key attributes in traces:
- `workflow.id`: Workflow identifier
- `http.method`: HTTP method
- `http.status_code`: Response status
- `db.statement`: SQL query
- `llm.provider`: LLM provider name
- `llm.model`: Model used
- `llm.tokens`: Token count

---
**Last Updated**: 2025-11-14
**Version**: 1.0
