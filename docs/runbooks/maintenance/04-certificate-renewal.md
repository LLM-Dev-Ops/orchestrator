# Certificate Renewal

## Overview
Renew TLS certificates before expiration to maintain secure HTTPS access.

## Step-by-Step Procedure

### Check Certificate Expiry

```bash
# Check current certificate
kubectl get certificate orchestrator-tls -n llm-orchestrator

# Get expiry date
echo | openssl s_client -servername orchestrator.example.com \
  -connect orchestrator.example.com:443 2>/dev/null | \
  openssl x509 -noout -dates

# Alert if expiring in < 30 days
```

### Renew with cert-manager (Automated)

```bash
# cert-manager should auto-renew at 30 days
# Force renewal if needed
kubectl delete certificate orchestrator-tls -n llm-orchestrator

# Recreate (cert-manager will issue new cert)
kubectl apply -f - <<EOF
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: orchestrator-tls
  namespace: llm-orchestrator
spec:
  secretName: orchestrator-tls
  issuerRef:
    name: letsencrypt-prod
    kind: ClusterIssuer
  dnsNames:
  - orchestrator.example.com
EOF

# Wait for issuance
kubectl wait --for=condition=ready certificate/orchestrator-tls -n llm-orchestrator --timeout=300s
```

### Manual Renewal

```bash
# Generate new certificate (if not using cert-manager)
# Using Let's Encrypt certbot:
certbot certonly --dns-route53 -d orchestrator.example.com

# Update secret
kubectl create secret tls orchestrator-tls-new \
  --cert=/etc/letsencrypt/live/orchestrator.example.com/fullchain.pem \
  --key=/etc/letsencrypt/live/orchestrator.example.com/privkey.pem \
  -n llm-orchestrator

# Update ingress
kubectl patch ingress orchestrator -n llm-orchestrator -p '
{
  "spec": {
    "tls": [{
      "hosts": ["orchestrator.example.com"],
      "secretName": "orchestrator-tls-new"
    }]
  }
}'

# Verify
curl -v https://orchestrator.example.com/health 2>&1 | grep "SSL certificate verify"
```

## Validation

```bash
# Certificate valid
openssl s_client -servername orchestrator.example.com \
  -connect orchestrator.example.com:443 < /dev/null 2>&1 | \
  openssl x509 -noout -dates

# HTTPS working
curl https://orchestrator.example.com/health
```

---
**Last Updated**: 2025-11-14
**Version**: 1.0
