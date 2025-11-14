-- Checkpoints table for workflow recovery

CREATE TABLE IF NOT EXISTS checkpoints (
    id UUID PRIMARY KEY,
    workflow_state_id UUID NOT NULL,
    step_id VARCHAR(255) NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    snapshot TEXT NOT NULL, -- JSON stored as TEXT
    FOREIGN KEY (workflow_state_id) REFERENCES workflow_states(id) ON DELETE CASCADE
);

-- Index for finding checkpoints by workflow and timestamp
CREATE INDEX IF NOT EXISTS idx_checkpoint_workflow_timestamp
    ON checkpoints(workflow_state_id, timestamp DESC);

-- Index for faster checkpoint lookups
CREATE INDEX IF NOT EXISTS idx_checkpoint_id ON checkpoints(id);
