#!/bin/bash
# Copyright (c) 2025 LLM DevOps
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Backup verification script

set -euo pipefail

BACKUP_DIR="${BACKUP_DIR:-/backups/llm-orchestrator}"

log() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $*"
}

error() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $*" >&2
}

usage() {
    cat <<EOF
Usage: $0 <backup-file>

Verify backup integrity without restoring.

EXAMPLES:
    $0 /backups/llm-orchestrator-backup-20250114_120000.tar.gz
EOF
    exit 1
}

if [ $# -lt 1 ]; then
    usage
fi

BACKUP_FILE="$1"

if [ ! -f "${BACKUP_FILE}" ]; then
    error "Backup file not found: ${BACKUP_FILE}"
    exit 1
fi

log "Verifying backup: ${BACKUP_FILE}"

# Extract to temp directory
TEMP_DIR=$(mktemp -d)
trap "rm -rf ${TEMP_DIR}" EXIT

log "Extracting backup..."
tar -xzf "${BACKUP_FILE}" -C "${TEMP_DIR}"

BACKUP_NAME=$(ls "${TEMP_DIR}" | head -1)
BACKUP_PATH="${TEMP_DIR}/${BACKUP_NAME}"

# Verify structure
log "Verifying backup structure..."
REQUIRED_FILES=("database.dump" "metadata.json" "checksums.sha256")
MISSING_FILES=()

for file in "${REQUIRED_FILES[@]}"; do
    if [ ! -f "${BACKUP_PATH}/${file}" ]; then
        MISSING_FILES+=("${file}")
    fi
done

if [ ${#MISSING_FILES[@]} -gt 0 ]; then
    error "Missing required files: ${MISSING_FILES[*]}"
    exit 1
fi

log "✓ Backup structure is valid"

# Verify checksums
log "Verifying checksums..."
cd "${BACKUP_PATH}"
if sha256sum -c checksums.sha256 --quiet; then
    log "✓ Checksum verification passed"
else
    error "Checksum verification failed"
    exit 1
fi
cd - > /dev/null

# Display metadata
log "Backup metadata:"
cat "${BACKUP_PATH}/metadata.json" | jq '.' 2>/dev/null || cat "${BACKUP_PATH}/metadata.json"

# Verify database dump
log "Verifying database dump..."
if pg_restore --list "${BACKUP_PATH}/database.dump" > /dev/null 2>&1; then
    log "✓ Database dump is valid"

    # Count tables
    TABLE_COUNT=$(pg_restore --list "${BACKUP_PATH}/database.dump" | grep -c "TABLE DATA" || echo "0")
    log "  Tables in backup: ${TABLE_COUNT}"
else
    error "Database dump is corrupted"
    exit 1
fi

# File size check
DUMP_SIZE=$(stat -c%s "${BACKUP_PATH}/database.dump")
if [ ${DUMP_SIZE} -lt 1024 ]; then
    error "Database dump suspiciously small: ${DUMP_SIZE} bytes"
    exit 1
fi
log "  Database dump size: $(numfmt --to=iec ${DUMP_SIZE})"

log ""
log "========================================"
log "✓ Backup verification PASSED"
log "========================================"
log "Backup is valid and can be restored"

exit 0
