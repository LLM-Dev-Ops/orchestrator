-- LLM Orchestrator Audit Events Table
-- Migration: 001_initial_schema
-- Description: Create the audit_events table with indexes for efficient querying

-- Create audit_events table
CREATE TABLE IF NOT EXISTS audit_events (
    -- Primary identifier
    id UUID PRIMARY KEY,

    -- Temporal information
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,

    -- Event classification
    event_type VARCHAR(100) NOT NULL,

    -- Actor information
    user_id VARCHAR(255),

    -- Action description
    action VARCHAR(255) NOT NULL,

    -- Resource information
    resource_type VARCHAR(50) NOT NULL,
    resource_id VARCHAR(255) NOT NULL,

    -- Result information
    result VARCHAR(50) NOT NULL,
    result_error TEXT,

    -- Additional details (structured data)
    details JSONB,

    -- Request context
    ip_address INET,
    user_agent TEXT,
    request_id VARCHAR(255),

    -- Tamper detection (hash chain)
    previous_hash VARCHAR(64),
    event_hash VARCHAR(64)
);

-- Create indexes for efficient querying

-- Index for time-based queries (most common)
CREATE INDEX IF NOT EXISTS idx_audit_timestamp
ON audit_events(timestamp DESC);

-- Index for user-based queries
CREATE INDEX IF NOT EXISTS idx_audit_user_id
ON audit_events(user_id)
WHERE user_id IS NOT NULL;

-- Index for event type filtering
CREATE INDEX IF NOT EXISTS idx_audit_event_type
ON audit_events(event_type);

-- Composite index for resource lookups
CREATE INDEX IF NOT EXISTS idx_audit_resource
ON audit_events(resource_type, resource_id);

-- Index for result-based queries
CREATE INDEX IF NOT EXISTS idx_audit_result
ON audit_events(result);

-- Index for request correlation
CREATE INDEX IF NOT EXISTS idx_audit_request_id
ON audit_events(request_id)
WHERE request_id IS NOT NULL;

-- GIN index for JSONB details queries
CREATE INDEX IF NOT EXISTS idx_audit_details
ON audit_events USING GIN (details);

-- Comments for documentation
COMMENT ON TABLE audit_events IS 'Audit log for all security and operational events in the LLM Orchestrator';
COMMENT ON COLUMN audit_events.id IS 'Unique identifier for the audit event';
COMMENT ON COLUMN audit_events.timestamp IS 'When the event occurred';
COMMENT ON COLUMN audit_events.event_type IS 'Type of event (authentication, authorization, workflow_execution, etc.)';
COMMENT ON COLUMN audit_events.user_id IS 'ID of the user who performed the action';
COMMENT ON COLUMN audit_events.action IS 'Human-readable description of the action';
COMMENT ON COLUMN audit_events.resource_type IS 'Type of resource affected (workflow, user, api_key, etc.)';
COMMENT ON COLUMN audit_events.resource_id IS 'Unique identifier of the affected resource';
COMMENT ON COLUMN audit_events.result IS 'Result of the action (success, failure, partial_success)';
COMMENT ON COLUMN audit_events.result_error IS 'Error message if the action failed';
COMMENT ON COLUMN audit_events.details IS 'Additional structured data about the event';
COMMENT ON COLUMN audit_events.ip_address IS 'IP address of the client';
COMMENT ON COLUMN audit_events.user_agent IS 'User agent string from the client';
COMMENT ON COLUMN audit_events.request_id IS 'Request ID for correlation across systems';
COMMENT ON COLUMN audit_events.previous_hash IS 'Hash of the previous audit event (for tamper detection)';
COMMENT ON COLUMN audit_events.event_hash IS 'Hash of this audit event (for tamper detection)';
