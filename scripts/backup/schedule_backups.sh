#!/bin/bash
# Copyright (c) 2025 LLM DevOps
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Setup automated backup schedule

set -euo pipefail

log() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $*"
}

error() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $*" >&2
}

# Determine script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKUP_SCRIPT="${SCRIPT_DIR}/backup.sh"

if [ ! -f "${BACKUP_SCRIPT}" ]; then
    error "Backup script not found: ${BACKUP_SCRIPT}"
    exit 1
fi

log "Setting up automated backup schedule..."

# Check if running as root or with sudo
if [ "$EUID" -ne 0 ]; then
    error "Please run as root or with sudo"
    exit 1
fi

# Create backup cron job
CRON_USER="${CRON_USER:-root}"
BACKUP_SCHEDULE="${BACKUP_SCHEDULE:-0 2 * * *}"  # Default: 2 AM daily

log "Schedule: ${BACKUP_SCHEDULE}"
log "User: ${CRON_USER}"

# Create cron job
CRON_JOB="${BACKUP_SCHEDULE} ${BACKUP_SCRIPT} >> /var/log/llm-orchestrator-backup.log 2>&1"

# Check if job already exists
if crontab -u "${CRON_USER}" -l 2>/dev/null | grep -q "${BACKUP_SCRIPT}"; then
    log "Backup job already exists in crontab"
    read -p "Update existing job? (yes/no): " -r
    if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
        log "Cancelled"
        exit 0
    fi

    # Remove old job
    crontab -u "${CRON_USER}" -l | grep -v "${BACKUP_SCRIPT}" | crontab -u "${CRON_USER}" -
fi

# Add new job
(crontab -u "${CRON_USER}" -l 2>/dev/null; echo "${CRON_JOB}") | crontab -u "${CRON_USER}" -

log "✓ Backup job added to crontab"

# Create log file
touch /var/log/llm-orchestrator-backup.log
chmod 644 /var/log/llm-orchestrator-backup.log

# Setup logrotate
LOGROTATE_CONF="/etc/logrotate.d/llm-orchestrator-backup"
cat > "${LOGROTATE_CONF}" <<EOF
/var/log/llm-orchestrator-backup.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 644 root root
}
EOF

log "✓ Logrotate configured"

# Create systemd timer (alternative to cron)
if command -v systemctl &> /dev/null; then
    log "Creating systemd timer..."

    # Service file
    cat > /etc/systemd/system/llm-orchestrator-backup.service <<EOF
[Unit]
Description=LLM Orchestrator Database Backup
After=network.target postgresql.service

[Service]
Type=oneshot
ExecStart=${BACKUP_SCRIPT}
User=${CRON_USER}
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

    # Timer file
    cat > /etc/systemd/system/llm-orchestrator-backup.timer <<EOF
[Unit]
Description=LLM Orchestrator Backup Timer
Requires=llm-orchestrator-backup.service

[Timer]
OnCalendar=daily
OnCalendar=02:00
Persistent=true

[Install]
WantedBy=timers.target
EOF

    # Reload and enable
    systemctl daemon-reload
    systemctl enable llm-orchestrator-backup.timer
    systemctl start llm-orchestrator-backup.timer

    log "✓ Systemd timer created and enabled"
    log ""
    log "Timer status:"
    systemctl status llm-orchestrator-backup.timer --no-pager
fi

# Summary
log ""
log "========================================"
log "Backup Schedule Configuration Complete"
log "========================================"
log ""
log "Cron Schedule: ${BACKUP_SCHEDULE}"
log "Backup Script: ${BACKUP_SCRIPT}"
log "Log File: /var/log/llm-orchestrator-backup.log"
log ""
log "View scheduled jobs:"
log "  crontab -u ${CRON_USER} -l"
log ""
log "View systemd timer:"
log "  systemctl list-timers llm-orchestrator-backup.timer"
log ""
log "View backup logs:"
log "  tail -f /var/log/llm-orchestrator-backup.log"
log ""
log "Test backup manually:"
log "  sudo ${BACKUP_SCRIPT}"

exit 0
