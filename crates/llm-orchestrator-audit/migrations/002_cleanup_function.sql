-- LLM Orchestrator Audit Events Cleanup Function
-- Migration: 002_cleanup_function
-- Description: Create function and trigger for automatic cleanup of old checkpoints

-- Function to cleanup old audit events based on retention policy
CREATE OR REPLACE FUNCTION cleanup_old_audit_events(retention_days INTEGER)
RETURNS TABLE(deleted_count BIGINT) AS $$
BEGIN
    RETURN QUERY
    WITH deleted AS (
        DELETE FROM audit_events
        WHERE timestamp < (NOW() - (retention_days || ' days')::INTERVAL)
        RETURNING *
    )
    SELECT COUNT(*)::BIGINT FROM deleted;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION cleanup_old_audit_events(INTEGER) IS
'Delete audit events older than the specified number of days. Returns the count of deleted events.';

-- Example usage:
-- SELECT cleanup_old_audit_events(90);  -- Delete events older than 90 days
