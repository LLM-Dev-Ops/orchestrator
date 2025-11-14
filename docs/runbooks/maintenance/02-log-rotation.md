# Log Rotation

## Overview
Manage and rotate application and system logs to prevent disk exhaustion.

## Step-by-Step Procedure

### Configure Log Rotation

```bash
# Application logs in Kubernetes
# Use FluentBit/Fluentd for log aggregation
kubectl apply -f - <<EOF
apiVersion: v1
kind: ConfigMap
metadata:
  name: fluent-bit-config
  namespace: logging
data:
  fluent-bit.conf: |
    [OUTPUT]
        Name  s3
        Match *
        bucket llm-orchestrator-logs
        region us-east-1
        total_file_size 100M
        upload_timeout 10m
        store_dir /tmp/fluent-bit
EOF

# PostgreSQL log rotation
kubectl exec -n llm-orchestrator postgres-0 -- sh -c '
  cat > /etc/logrotate.d/postgresql <<EOFLOG
/var/lib/postgresql/data/log/*.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    create 0640 postgres postgres
}
EOFLOG
'
```

### Manual Log Cleanup

```bash
# Compress old logs
kubectl exec -n llm-orchestrator postgres-0 -- \
  find /var/lib/postgresql/data/log -name "*.log" -mtime +1 -exec gzip {} \;

# Delete very old logs
kubectl exec -n llm-orchestrator postgres-0 -- \
  find /var/lib/postgresql/data/log -name "*.gz" -mtime +7 -delete
```

## Validation

```bash
# Verify logs being rotated
kubectl exec -n llm-orchestrator postgres-0 -- ls -lh /var/lib/postgresql/data/log/
```

---
**Last Updated**: 2025-11-14
**Version**: 1.0
