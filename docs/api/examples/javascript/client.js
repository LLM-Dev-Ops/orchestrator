/**
 * LLM Orchestrator JavaScript Client
 *
 * Example usage of the LLM Orchestrator API with JavaScript/Node.js.
 */

const axios = require('axios');

class LLMOrchestratorClient {
  /**
   * Create a new client instance
   * @param {Object} config - Configuration object
   * @param {string} config.baseURL - Base URL of the API
   * @param {string} [config.accessToken] - JWT access token
   * @param {string} [config.apiKey] - API key
   */
  constructor(config = {}) {
    this.baseURL = config.baseURL || 'https://api.llm-orchestrator.io/api/v1';
    this.accessToken = config.accessToken;
    this.apiKey = config.apiKey;

    this.client = axios.create({
      baseURL: this.baseURL,
      headers: this._getAuthHeaders(),
    });

    // Add response interceptor for error handling
    this.client.interceptors.response.use(
      (response) => response,
      (error) => this._handleError(error)
    );
  }

  _getAuthHeaders() {
    const headers = {
      'Content-Type': 'application/json',
    };

    if (this.accessToken) {
      headers['Authorization'] = `Bearer ${this.accessToken}`;
    } else if (this.apiKey) {
      headers['X-API-Key'] = this.apiKey;
    }

    return headers;
  }

  _updateAuthHeaders() {
    this.client.defaults.headers = {
      ...this.client.defaults.headers,
      ...this._getAuthHeaders(),
    };
  }

  _handleError(error) {
    if (error.response) {
      // API returned an error response
      const apiError = new Error(error.response.data.message || 'API Error');
      apiError.code = error.response.data.code;
      apiError.status = error.response.status;
      apiError.details = error.response.data.details;
      apiError.requestId = error.response.data.request_id;
      throw apiError;
    } else if (error.request) {
      // Request made but no response received
      throw new Error('No response from server');
    } else {
      // Error in request setup
      throw error;
    }
  }

  // ==================== Authentication ====================

  /**
   * Login with username and password
   * @param {string} username - Username
   * @param {string} password - Password
   * @returns {Promise<Object>} Login response with tokens
   */
  async login(username, password) {
    const response = await axios.post(`${this.baseURL}/auth/login`, {
      username,
      password,
    });

    this.accessToken = response.data.access_token;
    this._updateAuthHeaders();

    return response.data;
  }

  /**
   * Refresh JWT access token
   * @param {string} refreshToken - Refresh token
   * @returns {Promise<Object>} New access token
   */
  async refreshToken(refreshToken) {
    const response = await this.client.post('/auth/refresh', {
      refresh_token: refreshToken,
    });

    this.accessToken = response.data.access_token;
    this._updateAuthHeaders();

    return response.data;
  }

  /**
   * Create a new API key
   * @param {Object} params - API key parameters
   * @param {string} params.name - Key name
   * @param {string[]} params.scopes - Permission scopes
   * @param {number} [params.expiresInDays=90] - Expiration in days
   * @returns {Promise<Object>} Created API key
   */
  async createApiKey({ name, scopes, expiresInDays = 90 }) {
    const response = await this.client.post('/auth/keys', {
      name,
      scopes,
      expires_in_days: expiresInDays,
    });
    return response.data;
  }

  /**
   * List all API keys
   * @param {Object} [params] - Query parameters
   * @param {number} [params.limit=20] - Results per page
   * @param {number} [params.offset=0] - Results to skip
   * @returns {Promise<Object>} List of API keys
   */
  async listApiKeys({ limit = 20, offset = 0 } = {}) {
    const response = await this.client.get('/auth/keys', {
      params: { limit, offset },
    });
    return response.data;
  }

  /**
   * Revoke an API key
   * @param {string} keyId - API key ID
   * @returns {Promise<void>}
   */
  async revokeApiKey(keyId) {
    await this.client.delete(`/auth/keys/${keyId}`);
  }

  // ==================== Workflows ====================

  /**
   * Create a new workflow
   * @param {Object} workflow - Workflow definition
   * @returns {Promise<Object>} Created workflow
   */
  async createWorkflow(workflow) {
    const response = await this.client.post('/workflows', workflow);
    return response.data;
  }

  /**
   * List workflows
   * @param {Object} [params] - Query parameters
   * @param {number} [params.limit=20] - Results per page
   * @param {number} [params.offset=0] - Results to skip
   * @param {string} [params.name] - Filter by name
   * @param {string} [params.version] - Filter by version
   * @returns {Promise<Object>} List of workflows
   */
  async listWorkflows({ limit = 20, offset = 0, name, version } = {}) {
    const params = { limit, offset };
    if (name) params.name = name;
    if (version) params.version = version;

    const response = await this.client.get('/workflows', { params });
    return response.data;
  }

  /**
   * Get workflow details
   * @param {string} workflowId - Workflow ID
   * @returns {Promise<Object>} Workflow details
   */
  async getWorkflow(workflowId) {
    const response = await this.client.get(`/workflows/${workflowId}`);
    return response.data;
  }

  /**
   * Update workflow
   * @param {string} workflowId - Workflow ID
   * @param {Object} workflow - Updated workflow definition
   * @returns {Promise<Object>} Updated workflow
   */
  async updateWorkflow(workflowId, workflow) {
    const response = await this.client.put(`/workflows/${workflowId}`, workflow);
    return response.data;
  }

  /**
   * Delete workflow
   * @param {string} workflowId - Workflow ID
   * @returns {Promise<void>}
   */
  async deleteWorkflow(workflowId) {
    await this.client.delete(`/workflows/${workflowId}`);
  }

  // ==================== Execution ====================

  /**
   * Execute a workflow
   * @param {string} workflowId - Workflow ID
   * @param {Object} params - Execution parameters
   * @param {Object} params.inputs - Input variables
   * @param {boolean} [params.async=true] - Async execution
   * @param {number} [params.timeoutOverride] - Custom timeout
   * @returns {Promise<Object>} Execution response
   */
  async executeWorkflow(workflowId, { inputs, async = true, timeoutOverride } = {}) {
    const data = { inputs, async };
    if (timeoutOverride) data.timeout_override = timeoutOverride;

    const response = await this.client.post(
      `/workflows/${workflowId}/execute`,
      data
    );
    return response.data;
  }

  /**
   * Get execution status
   * @param {string} workflowId - Workflow ID
   * @param {string} executionId - Execution ID
   * @returns {Promise<Object>} Execution status
   */
  async getExecutionStatus(workflowId, executionId) {
    const response = await this.client.get(`/workflows/${workflowId}/status`, {
      params: { executionId },
    });
    return response.data;
  }

  /**
   * Pause workflow execution
   * @param {string} workflowId - Workflow ID
   * @param {string} executionId - Execution ID
   * @returns {Promise<Object>} Updated execution state
   */
  async pauseWorkflow(workflowId, executionId) {
    const response = await this.client.post(`/workflows/${workflowId}/pause`, null, {
      params: { executionId },
    });
    return response.data;
  }

  /**
   * Resume paused workflow
   * @param {string} workflowId - Workflow ID
   * @param {string} executionId - Execution ID
   * @returns {Promise<Object>} Updated execution state
   */
  async resumeWorkflow(workflowId, executionId) {
    const response = await this.client.post(`/workflows/${workflowId}/resume`, null, {
      params: { executionId },
    });
    return response.data;
  }

  /**
   * Cancel workflow execution
   * @param {string} workflowId - Workflow ID
   * @param {string} executionId - Execution ID
   * @returns {Promise<Object>} Updated execution state
   */
  async cancelWorkflow(workflowId, executionId) {
    const response = await this.client.post(`/workflows/${workflowId}/cancel`, null, {
      params: { executionId },
    });
    return response.data;
  }

  /**
   * Wait for workflow completion
   * @param {string} workflowId - Workflow ID
   * @param {string} executionId - Execution ID
   * @param {Object} [options] - Polling options
   * @param {number} [options.pollInterval=2000] - Poll interval in ms
   * @param {number} [options.maxAttempts=60] - Maximum attempts
   * @returns {Promise<Object>} Final execution state
   */
  async waitForCompletion(
    workflowId,
    executionId,
    { pollInterval = 2000, maxAttempts = 60 } = {}
  ) {
    for (let attempt = 0; attempt < maxAttempts; attempt++) {
      const status = await this.getExecutionStatus(workflowId, executionId);

      if (status.status === 'completed' || status.status === 'failed') {
        return status;
      }

      await new Promise((resolve) => setTimeout(resolve, pollInterval));
    }

    throw new Error(
      `Workflow execution did not complete within ${
        (maxAttempts * pollInterval) / 1000
      } seconds`
    );
  }

  // ==================== State Management ====================

  /**
   * Get workflow state
   * @param {string} workflowId - Workflow ID
   * @param {string} executionId - Execution ID
   * @returns {Promise<Object>} Workflow state
   */
  async getWorkflowState(workflowId, executionId) {
    const response = await this.client.get(`/state/${workflowId}`, {
      params: { executionId },
    });
    return response.data;
  }

  /**
   * List checkpoints
   * @param {string} workflowId - Workflow ID
   * @param {string} executionId - Execution ID
   * @param {Object} [params] - Query parameters
   * @param {number} [params.limit=20] - Results per page
   * @param {number} [params.offset=0] - Results to skip
   * @returns {Promise<Object>} List of checkpoints
   */
  async listCheckpoints(workflowId, executionId, { limit = 20, offset = 0 } = {}) {
    const response = await this.client.get(`/state/${workflowId}/checkpoints`, {
      params: { executionId, limit, offset },
    });
    return response.data;
  }

  /**
   * Create a checkpoint
   * @param {string} workflowId - Workflow ID
   * @param {string} executionId - Execution ID
   * @param {string} stepId - Step ID
   * @returns {Promise<Object>} Created checkpoint
   */
  async createCheckpoint(workflowId, executionId, stepId) {
    const response = await this.client.post(`/state/${workflowId}/checkpoints`, {
      executionId,
      stepId,
    });
    return response.data;
  }

  /**
   * Restore from checkpoint
   * @param {string} workflowId - Workflow ID
   * @param {string} checkpointId - Checkpoint ID
   * @returns {Promise<Object>} Restored execution
   */
  async restoreCheckpoint(workflowId, checkpointId) {
    const response = await this.client.post(`/state/${workflowId}/restore`, {
      checkpointId,
    });
    return response.data;
  }

  // ==================== Monitoring ====================

  /**
   * Get health status
   * @returns {Promise<Object>} Health status
   */
  async healthCheck() {
    const response = await axios.get(`${this.baseURL}/health`);
    return response.data;
  }

  /**
   * Check if service is ready
   * @returns {Promise<boolean>} True if ready
   */
  async readinessProbe() {
    try {
      const response = await axios.get(`${this.baseURL}/health/ready`);
      return response.status === 200;
    } catch {
      return false;
    }
  }

  /**
   * Check if service is alive
   * @returns {Promise<boolean>} True if alive
   */
  async livenessProbe() {
    try {
      const response = await axios.get(`${this.baseURL}/health/live`);
      return response.status === 200;
    } catch {
      return false;
    }
  }

  /**
   * Get Prometheus metrics
   * @returns {Promise<string>} Metrics in Prometheus format
   */
  async getMetrics() {
    const response = await axios.get(`${this.baseURL}/metrics`);
    return response.data;
  }

  // ==================== Audit ====================

  /**
   * Query audit events
   * @param {Object} [params] - Query parameters
   * @returns {Promise<Object>} List of audit events
   */
  async queryAuditEvents(params = {}) {
    const response = await this.client.get('/audit/events', { params });
    return response.data;
  }

  /**
   * Get audit event details
   * @param {string} eventId - Event ID
   * @returns {Promise<Object>} Event details
   */
  async getAuditEvent(eventId) {
    const response = await this.client.get(`/audit/events/${eventId}`);
    return response.data;
  }

  // ==================== Admin ====================

  /**
   * List users (admin only)
   * @param {Object} [params] - Query parameters
   * @returns {Promise<Object>} List of users
   */
  async listUsers({ limit = 20, offset = 0 } = {}) {
    const response = await this.client.get('/admin/users', {
      params: { limit, offset },
    });
    return response.data;
  }

  /**
   * Create user (admin only)
   * @param {Object} user - User data
   * @returns {Promise<Object>} Created user
   */
  async createUser(user) {
    const response = await this.client.post('/admin/users', user);
    return response.data;
  }

  /**
   * Get system statistics (admin only)
   * @returns {Promise<Object>} System stats
   */
  async getSystemStats() {
    const response = await this.client.get('/admin/stats');
    return response.data;
  }

  /**
   * Manage secret (admin only)
   * @param {Object} params - Secret parameters
   * @returns {Promise<Object>} Response
   */
  async manageSecret({ key, value, ttlSeconds }) {
    const data = { key, value };
    if (ttlSeconds) data.ttl_seconds = ttlSeconds;

    const response = await this.client.post('/admin/secrets', data);
    return response.data;
  }

  /**
   * Get system configuration (admin only)
   * @returns {Promise<Object>} System config
   */
  async getSystemConfig() {
    const response = await this.client.get('/admin/config');
    return response.data;
  }
}

// ==================== Example Usage ====================

async function main() {
  try {
    // Initialize client
    const client = new LLMOrchestratorClient({
      baseURL: 'https://api.llm-orchestrator.io/api/v1',
    });

    // 1. Login
    console.log('1. Logging in...');
    const loginResponse = await client.login('admin', 'secure_password');
    console.log(`   Logged in as: ${loginResponse.user_id}`);
    console.log(`   Roles: ${loginResponse.roles.join(', ')}`);

    // 2. Create workflow
    console.log('\n2. Creating workflow...');
    const workflow = {
      name: 'sentiment-analyzer',
      version: '1.0',
      description: 'Analyzes sentiment of input text',
      steps: [
        {
          id: 'analyze',
          type: 'llm',
          provider: 'openai',
          model: 'gpt-4',
          prompt: 'Analyze sentiment: {{input}}',
          temperature: 0.3,
          max_tokens: 50,
          output: ['sentiment'],
        },
      ],
    };

    const workflowResponse = await client.createWorkflow(workflow);
    const workflowId = workflowResponse.id;
    console.log(`   Created workflow: ${workflowId}`);

    // 3. Execute workflow
    console.log('\n3. Executing workflow...');
    const execution = await client.executeWorkflow(workflowId, {
      inputs: { input: 'This is amazing!' },
    });
    const executionId = execution.execution_id;
    console.log(`   Execution started: ${executionId}`);

    // 4. Wait for completion
    console.log('\n4. Waiting for completion...');
    const result = await client.waitForCompletion(workflowId, executionId);
    console.log(`   Status: ${result.status}`);
    if (result.status === 'completed') {
      console.log(`   Outputs: ${JSON.stringify(result.context?.outputs || {})}`);
    }

    // 5. List workflows
    console.log('\n5. Listing workflows...');
    const workflows = await client.listWorkflows({ limit: 5 });
    console.log(`   Total workflows: ${workflows.total}`);

    // 6. Health check
    console.log('\n6. Health check...');
    const health = await client.healthCheck();
    console.log(`   Status: ${health.status}`);

    console.log('\n✓ Examples completed successfully!');
  } catch (error) {
    console.error('Error:', error.message);
    if (error.code) console.error('Code:', error.code);
    if (error.requestId) console.error('Request ID:', error.requestId);
    process.exit(1);
  }
}

// Export the client
module.exports = LLMOrchestratorClient;

// Run examples if executed directly
if (require.main === module) {
  main();
}
