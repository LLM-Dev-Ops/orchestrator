#!/bin/bash
# Copyright (c) 2025 LLM DevOps
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Restore script for LLM Orchestrator backups

set -euo pipefail

# Configuration
BACKUP_DIR="${BACKUP_DIR:-/backups/llm-orchestrator}"
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-workflows}"
DB_USER="${DB_USER:-postgres}"
PGPASSWORD="${PGPASSWORD:-}"

# Logging
log() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $*"
}

error() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $*" >&2
}

# Usage
usage() {
    cat <<EOF
Usage: $0 [OPTIONS] <backup-file>

Restore LLM Orchestrator from backup.

OPTIONS:
    -h, --help              Show this help message
    -f, --force             Force restore without confirmation
    -d, --download          Download from S3 before restore
    -v, --verify            Verify backup integrity before restore

EXAMPLES:
    # Restore from local backup
    $0 /backups/llm-orchestrator-backup-20250114_120000.tar.gz

    # Download from S3 and restore
    $0 --download s3://my-bucket/backups/llm-orchestrator-backup-20250114_120000.tar.gz

    # Verify and restore
    $0 --verify /backups/llm-orchestrator-backup-20250114_120000.tar.gz
EOF
    exit 1
}

# Parse arguments
FORCE=false
DOWNLOAD=false
VERIFY=false
BACKUP_FILE=""

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            usage
            ;;
        -f|--force)
            FORCE=true
            shift
            ;;
        -d|--download)
            DOWNLOAD=true
            shift
            ;;
        -v|--verify)
            VERIFY=true
            shift
            ;;
        *)
            BACKUP_FILE="$1"
            shift
            ;;
    esac
done

if [ -z "${BACKUP_FILE}" ]; then
    error "No backup file specified"
    usage
fi

log "Starting restore from: ${BACKUP_FILE}"

# Download from S3 if needed
if [ "${DOWNLOAD}" = true ]; then
    log "Downloading backup from S3..."
    LOCAL_BACKUP="${BACKUP_DIR}/$(basename ${BACKUP_FILE})"
    aws s3 cp "${BACKUP_FILE}" "${LOCAL_BACKUP}"
    BACKUP_FILE="${LOCAL_BACKUP}"
fi

# Verify backup file exists
if [ ! -f "${BACKUP_FILE}" ]; then
    error "Backup file not found: ${BACKUP_FILE}"
    exit 1
fi

# Extract backup
TEMP_DIR=$(mktemp -d)
trap "rm -rf ${TEMP_DIR}" EXIT

log "Extracting backup..."
tar -xzf "${BACKUP_FILE}" -C "${TEMP_DIR}"

# Find the backup directory
BACKUP_NAME=$(ls "${TEMP_DIR}" | head -1)
BACKUP_PATH="${TEMP_DIR}/${BACKUP_NAME}"

# Verify checksums
if [ "${VERIFY}" = true ] || [ -f "${BACKUP_PATH}/checksums.sha256" ]; then
    log "Verifying backup integrity..."
    cd "${BACKUP_PATH}"
    if sha256sum -c checksums.sha256 --quiet; then
        log "Checksum verification passed"
    else
        error "Checksum verification failed"
        exit 1
    fi
    cd - > /dev/null
fi

# Read metadata
if [ -f "${BACKUP_PATH}/metadata.json" ]; then
    log "Backup metadata:"
    cat "${BACKUP_PATH}/metadata.json"
fi

# Confirmation prompt
if [ "${FORCE}" = false ]; then
    echo ""
    echo "WARNING: This will DROP and RECREATE the database '${DB_NAME}'"
    echo "         All existing data will be lost!"
    echo ""
    read -p "Are you sure you want to continue? (yes/no): " -r
    if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
        log "Restore cancelled by user"
        exit 0
    fi
fi

# Stop application (if running in Docker)
log "Stopping application..."
docker-compose stop orchestrator 2>/dev/null || true

# Drop and recreate database
log "Dropping existing database..."
psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -c "DROP DATABASE IF EXISTS ${DB_NAME};" postgres

log "Creating new database..."
psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -c "CREATE DATABASE ${DB_NAME};" postgres

# Restore database
log "Restoring database..."
pg_restore \
    -h "${DB_HOST}" \
    -p "${DB_PORT}" \
    -U "${DB_USER}" \
    -d "${DB_NAME}" \
    -v \
    "${BACKUP_PATH}/database.dump" \
    2>&1 | tee "${BACKUP_DIR}/restore-$(date +%Y%m%d_%H%M%S).log"

if [ ${PIPESTATUS[0]} -eq 0 ]; then
    log "Database restore completed successfully"
else
    error "Database restore failed"
    exit 1
fi

# Verify database
log "Verifying database..."
WORKFLOW_COUNT=$(psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d "${DB_NAME}" -t -c "SELECT COUNT(*) FROM workflow_states;" | tr -d ' ')
log "Workflow states in database: ${WORKFLOW_COUNT}"

# Restart application
log "Restarting application..."
docker-compose start orchestrator 2>/dev/null || true

# Wait for health check
log "Waiting for application health check..."
sleep 5

log "Restore completed successfully!"
log "Please verify application functionality"

exit 0
