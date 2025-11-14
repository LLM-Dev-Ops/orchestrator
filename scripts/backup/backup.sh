#!/bin/bash
# Copyright (c) 2025 LLM DevOps
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Database and state backup script for LLM Orchestrator

set -euo pipefail

# Configuration
BACKUP_DIR="${BACKUP_DIR:-/backups/llm-orchestrator}"
RETENTION_DAYS="${RETENTION_DAYS:-30}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_NAME="llm-orchestrator-backup-${TIMESTAMP}"

# Database connection (override via environment)
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-workflows}"
DB_USER="${DB_USER:-postgres}"
PGPASSWORD="${PGPASSWORD:-}"

# S3 configuration (optional)
S3_BUCKET="${S3_BUCKET:-}"
S3_PREFIX="${S3_PREFIX:-backups/llm-orchestrator}"

# Logging
log() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $*"
}

error() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $*" >&2
}

# Create backup directory
mkdir -p "${BACKUP_DIR}"
BACKUP_PATH="${BACKUP_DIR}/${BACKUP_NAME}"
mkdir -p "${BACKUP_PATH}"

log "Starting backup: ${BACKUP_NAME}"

# 1. Backup PostgreSQL database
log "Backing up PostgreSQL database..."
pg_dump \
    -h "${DB_HOST}" \
    -p "${DB_PORT}" \
    -U "${DB_USER}" \
    -d "${DB_NAME}" \
    -Fc \
    -f "${BACKUP_PATH}/database.dump" \
    2>&1 | tee "${BACKUP_PATH}/pg_dump.log"

if [ ${PIPESTATUS[0]} -eq 0 ]; then
    log "Database backup completed successfully"
    DB_SIZE=$(du -h "${BACKUP_PATH}/database.dump" | cut -f1)
    log "Database backup size: ${DB_SIZE}"
else
    error "Database backup failed"
    exit 1
fi

# 2. Export backup metadata
log "Creating backup metadata..."
cat > "${BACKUP_PATH}/metadata.json" <<EOF
{
    "backup_name": "${BACKUP_NAME}",
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "database": {
        "host": "${DB_HOST}",
        "port": ${DB_PORT},
        "name": "${DB_NAME}",
        "size_bytes": $(stat -c%s "${BACKUP_PATH}/database.dump")
    },
    "version": "1.0",
    "retention_days": ${RETENTION_DAYS}
}
EOF

# 3. Calculate checksums
log "Calculating checksums..."
cd "${BACKUP_PATH}"
sha256sum database.dump > checksums.sha256
cd - > /dev/null

# 4. Create compressed archive
log "Creating compressed archive..."
tar -czf "${BACKUP_DIR}/${BACKUP_NAME}.tar.gz" -C "${BACKUP_DIR}" "${BACKUP_NAME}"
ARCHIVE_SIZE=$(du -h "${BACKUP_DIR}/${BACKUP_NAME}.tar.gz" | cut -f1)
log "Archive size: ${ARCHIVE_SIZE}"

# 5. Upload to S3 (if configured)
if [ -n "${S3_BUCKET}" ]; then
    log "Uploading to S3: s3://${S3_BUCKET}/${S3_PREFIX}/${BACKUP_NAME}.tar.gz"
    aws s3 cp \
        "${BACKUP_DIR}/${BACKUP_NAME}.tar.gz" \
        "s3://${S3_BUCKET}/${S3_PREFIX}/${BACKUP_NAME}.tar.gz" \
        --storage-class STANDARD_IA \
        --metadata "backup-timestamp=${TIMESTAMP},retention-days=${RETENTION_DAYS}"

    if [ $? -eq 0 ]; then
        log "S3 upload successful"
    else
        error "S3 upload failed"
    fi
fi

# 6. Clean up old backups
log "Cleaning up old backups (older than ${RETENTION_DAYS} days)..."
find "${BACKUP_DIR}" -name "llm-orchestrator-backup-*.tar.gz" -mtime +${RETENTION_DAYS} -delete
find "${BACKUP_DIR}" -name "llm-orchestrator-backup-*" -type d -mtime +${RETENTION_DAYS} -exec rm -rf {} + 2>/dev/null || true

# 7. Clean up temporary backup directory
rm -rf "${BACKUP_PATH}"

# 8. Create backup report
REPORT_FILE="${BACKUP_DIR}/backup-report-${TIMESTAMP}.txt"
cat > "${REPORT_FILE}" <<EOF
Backup Report
=============
Backup Name: ${BACKUP_NAME}
Timestamp: $(date)
Status: SUCCESS

Database:
- Host: ${DB_HOST}:${DB_PORT}
- Database: ${DB_NAME}
- Size: ${DB_SIZE}

Archive:
- Location: ${BACKUP_DIR}/${BACKUP_NAME}.tar.gz
- Size: ${ARCHIVE_SIZE}

S3 Upload: $([ -n "${S3_BUCKET}" ] && echo "YES" || echo "NO")
$([ -n "${S3_BUCKET}" ] && echo "- Bucket: s3://${S3_BUCKET}/${S3_PREFIX}/${BACKUP_NAME}.tar.gz")

Retention: ${RETENTION_DAYS} days
EOF

log "Backup completed successfully"
log "Report: ${REPORT_FILE}"

exit 0
