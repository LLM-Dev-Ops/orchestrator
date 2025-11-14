-- Initial schema for workflow state persistence

-- Workflow states table
CREATE TABLE IF NOT EXISTS workflow_states (
    id UUID PRIMARY KEY,
    workflow_id VARCHAR(255) NOT NULL,
    workflow_name VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL,
    user_id VARCHAR(255),
    started_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    completed_at TIMESTAMP WITH TIME ZONE,
    context TEXT NOT NULL, -- JSON stored as TEXT for SQLite compatibility
    error TEXT
);

-- Indexes for workflow_states
CREATE INDEX IF NOT EXISTS idx_workflow_id_status ON workflow_states(workflow_id, status);
CREATE INDEX IF NOT EXISTS idx_status ON workflow_states(status);
CREATE INDEX IF NOT EXISTS idx_user_id ON workflow_states(user_id);
CREATE INDEX IF NOT EXISTS idx_updated_at ON workflow_states(updated_at DESC);

-- Step states table
CREATE TABLE IF NOT EXISTS step_states (
    workflow_state_id UUID NOT NULL,
    step_id VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL,
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    outputs TEXT, -- JSON stored as TEXT
    error TEXT,
    retry_count INTEGER DEFAULT 0,
    PRIMARY KEY (workflow_state_id, step_id),
    FOREIGN KEY (workflow_state_id) REFERENCES workflow_states(id) ON DELETE CASCADE
);

-- Index for step_states
CREATE INDEX IF NOT EXISTS idx_step_workflow_state ON step_states(workflow_state_id);
