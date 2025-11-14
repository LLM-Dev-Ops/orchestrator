// LLM Orchestrator Go Client
//
// Example usage of the LLM Orchestrator API with Go.

package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"
)

// ApiConfig holds API configuration
type ApiConfig struct {
	BaseURL     string
	AccessToken string
	APIKey      string
}

// DefaultConfig returns default configuration
func DefaultConfig() *ApiConfig {
	return &ApiConfig{
		BaseURL: "https://api.llm-orchestrator.io/api/v1",
	}
}

// Client is the LLM Orchestrator API client
type Client struct {
	config     *ApiConfig
	httpClient *http.Client
}

// NewClient creates a new API client
func NewClient(config *ApiConfig) *Client {
	return &Client{
		config: config,
		httpClient: &http.Client{
			Timeout: 30 * time.Second,
		},
	}
}

// doRequest performs an HTTP request with auth headers
func (c *Client) doRequest(method, path string, body interface{}) (*http.Response, error) {
	url := c.config.BaseURL + path

	var reqBody io.Reader
	if body != nil {
		jsonData, err := json.Marshal(body)
		if err != nil {
			return nil, err
		}
		reqBody = bytes.NewBuffer(jsonData)
	}

	req, err := http.NewRequest(method, url, reqBody)
	if err != nil {
		return nil, err
	}

	req.Header.Set("Content-Type", "application/json")

	// Set authentication headers
	if c.config.AccessToken != "" {
		req.Header.Set("Authorization", "Bearer "+c.config.AccessToken)
	} else if c.config.APIKey != "" {
		req.Header.Set("X-API-Key", c.config.APIKey)
	}

	return c.httpClient.Do(req)
}

// ==================== Authentication ====================

// LoginRequest is the login request payload
type LoginRequest struct {
	Username string `json:"username"`
	Password string `json:"password"`
}

// LoginResponse is the login response
type LoginResponse struct {
	AccessToken  string   `json:"access_token"`
	RefreshToken string   `json:"refresh_token"`
	TokenType    string   `json:"token_type"`
	ExpiresIn    int      `json:"expires_in"`
	UserID       string   `json:"user_id"`
	Roles        []string `json:"roles"`
}

// Login authenticates with username and password
func (c *Client) Login(username, password string) (*LoginResponse, error) {
	req := LoginRequest{
		Username: username,
		Password: password,
	}

	resp, err := c.doRequest("POST", "/auth/login", req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var loginResp LoginResponse
	if err := json.NewDecoder(resp.Body).Decode(&loginResp); err != nil {
		return nil, err
	}

	// Store access token
	c.config.AccessToken = loginResp.AccessToken

	return &loginResp, nil
}

// ==================== Workflows ====================

// Workflow represents a workflow definition
type Workflow struct {
	ID             string                 `json:"id,omitempty"`
	Name           string                 `json:"name"`
	Version        string                 `json:"version"`
	Description    string                 `json:"description,omitempty"`
	Steps          []map[string]interface{} `json:"steps"`
	TimeoutSeconds int                    `json:"timeout_seconds,omitempty"`
	Metadata       map[string]interface{} `json:"metadata,omitempty"`
}

// WorkflowList represents a list of workflows
type WorkflowList struct {
	Workflows []Workflow `json:"workflows"`
	Total     int        `json:"total"`
	Limit     int        `json:"limit"`
	Offset    int        `json:"offset"`
}

// CreateWorkflow creates a new workflow
func (c *Client) CreateWorkflow(workflow *Workflow) (*Workflow, error) {
	resp, err := c.doRequest("POST", "/workflows", workflow)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var created Workflow
	if err := json.NewDecoder(resp.Body).Decode(&created); err != nil {
		return nil, err
	}

	return &created, nil
}

// ListWorkflows lists all workflows
func (c *Client) ListWorkflows(limit, offset int) (*WorkflowList, error) {
	path := fmt.Sprintf("/workflows?limit=%d&offset=%d", limit, offset)

	resp, err := c.doRequest("GET", path, nil)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var list WorkflowList
	if err := json.NewDecoder(resp.Body).Decode(&list); err != nil {
		return nil, err
	}

	return &list, nil
}

// GetWorkflow gets workflow details
func (c *Client) GetWorkflow(workflowID string) (*Workflow, error) {
	path := fmt.Sprintf("/workflows/%s", workflowID)

	resp, err := c.doRequest("GET", path, nil)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var workflow Workflow
	if err := json.NewDecoder(resp.Body).Decode(&workflow); err != nil {
		return nil, err
	}

	return &workflow, nil
}

// ==================== Execution ====================

// ExecuteWorkflowRequest is the execution request
type ExecuteWorkflowRequest struct {
	Inputs          map[string]interface{} `json:"inputs"`
	Async           bool                   `json:"async"`
	TimeoutOverride int                    `json:"timeout_override,omitempty"`
}

// ExecutionResponse is the execution response
type ExecutionResponse struct {
	ExecutionID string                 `json:"execution_id"`
	WorkflowID  string                 `json:"workflow_id"`
	Status      string                 `json:"status"`
	StartedAt   string                 `json:"started_at"`
	CompletedAt string                 `json:"completed_at,omitempty"`
	Outputs     map[string]interface{} `json:"outputs,omitempty"`
}

// ExecuteWorkflow executes a workflow
func (c *Client) ExecuteWorkflow(workflowID string, inputs map[string]interface{}, async bool) (*ExecutionResponse, error) {
	path := fmt.Sprintf("/workflows/%s/execute", workflowID)

	req := ExecuteWorkflowRequest{
		Inputs: inputs,
		Async:  async,
	}

	resp, err := c.doRequest("POST", path, req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var execution ExecutionResponse
	if err := json.NewDecoder(resp.Body).Decode(&execution); err != nil {
		return nil, err
	}

	return &execution, nil
}

// GetExecutionStatus gets execution status
func (c *Client) GetExecutionStatus(workflowID, executionID string) (*ExecutionResponse, error) {
	path := fmt.Sprintf("/workflows/%s/status?executionId=%s", workflowID, executionID)

	resp, err := c.doRequest("GET", path, nil)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var status ExecutionResponse
	if err := json.NewDecoder(resp.Body).Decode(&status); err != nil {
		return nil, err
	}

	return &status, nil
}

// WaitForCompletion polls execution status until completion
func (c *Client) WaitForCompletion(workflowID, executionID string, pollInterval time.Duration, maxAttempts int) (*ExecutionResponse, error) {
	for i := 0; i < maxAttempts; i++ {
		status, err := c.GetExecutionStatus(workflowID, executionID)
		if err != nil {
			return nil, err
		}

		if status.Status == "completed" || status.Status == "failed" {
			return status, nil
		}

		time.Sleep(pollInterval)
	}

	return nil, fmt.Errorf("workflow execution timeout")
}

// ==================== Monitoring ====================

// HealthCheck performs a health check
func (c *Client) HealthCheck() (map[string]interface{}, error) {
	resp, err := c.doRequest("GET", "/health", nil)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var health map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&health); err != nil {
		return nil, err
	}

	return health, nil
}

// ==================== Example Usage ====================

func main() {
	fmt.Println("=== LLM Orchestrator Go Client Example ===\n")

	// Initialize client
	config := DefaultConfig()
	client := NewClient(config)

	// 1. Login
	fmt.Println("1. Logging in...")
	loginResp, err := client.Login("admin", "secure_password")
	if err != nil {
		fmt.Printf("Login failed: %v\n", err)
		return
	}
	fmt.Printf("   Logged in as: %s\n", loginResp.UserID)
	fmt.Printf("   Roles: %v\n", loginResp.Roles)

	// 2. Create workflow
	fmt.Println("\n2. Creating workflow...")
	workflow := &Workflow{
		Name:        "sentiment-analyzer",
		Version:     "1.0",
		Description: "Analyzes sentiment of input text",
		Steps: []map[string]interface{}{
			{
				"id":          "analyze",
				"type":        "llm",
				"provider":    "openai",
				"model":       "gpt-4",
				"prompt":      "Analyze sentiment: {{input}}",
				"temperature": 0.3,
				"max_tokens":  50,
				"output":      []string{"sentiment"},
			},
		},
		TimeoutSeconds: 300,
	}

	created, err := client.CreateWorkflow(workflow)
	if err != nil {
		fmt.Printf("Create workflow failed: %v\n", err)
		return
	}
	fmt.Printf("   Created workflow: %s\n", created.ID)

	// 3. Execute workflow
	fmt.Println("\n3. Executing workflow...")
	inputs := map[string]interface{}{
		"input": "This is amazing!",
	}

	execution, err := client.ExecuteWorkflow(created.ID, inputs, true)
	if err != nil {
		fmt.Printf("Execute workflow failed: %v\n", err)
		return
	}
	fmt.Printf("   Execution started: %s\n", execution.ExecutionID)

	// 4. Wait for completion
	fmt.Println("\n4. Waiting for completion...")
	result, err := client.WaitForCompletion(created.ID, execution.ExecutionID, 2*time.Second, 60)
	if err != nil {
		fmt.Printf("Wait failed: %v\n", err)
		return
	}
	fmt.Printf("   Status: %s\n", result.Status)

	// 5. List workflows
	fmt.Println("\n5. Listing workflows...")
	workflows, err := client.ListWorkflows(5, 0)
	if err != nil {
		fmt.Printf("List workflows failed: %v\n", err)
		return
	}
	fmt.Printf("   Total workflows: %d\n", workflows.Total)

	// 6. Health check
	fmt.Println("\n6. Health check...")
	health, err := client.HealthCheck()
	if err != nil {
		fmt.Printf("Health check failed: %v\n", err)
		return
	}
	fmt.Printf("   Status: %v\n", health["status"])

	fmt.Println("\n✓ Examples completed successfully!")
}
