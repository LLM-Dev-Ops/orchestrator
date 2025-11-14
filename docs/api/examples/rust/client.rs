// LLM Orchestrator Rust Client
//
// Example usage of the LLM Orchestrator API with Rust.

use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub base_url: String,
    pub access_token: Option<String>,
    pub api_key: Option<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.llm-orchestrator.io/api/v1".to_string(),
            access_token: None,
            api_key: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub user_id: String,
    pub roles: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Workflow {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub steps: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteWorkflowRequest {
    pub inputs: HashMap<String, Value>,
    #[serde(rename = "async")]
    pub async_exec: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_override: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionResponse {
    pub execution_id: String,
    pub workflow_id: String,
    pub status: String,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<HashMap<String, Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowList {
    pub workflows: Vec<Workflow>,
    pub total: u64,
    pub limit: u64,
    pub offset: u64,
}

pub struct LLMOrchestratorClient {
    config: ApiConfig,
    client: Client,
}

impl LLMOrchestratorClient {
    pub fn new(config: ApiConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    fn get_auth_headers(&self) -> header::HeaderMap {
        let mut headers = header::HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());

        if let Some(ref token) = self.config.access_token {
            headers.insert(
                header::AUTHORIZATION,
                format!("Bearer {}", token).parse().unwrap(),
            );
        } else if let Some(ref api_key) = self.config.api_key {
            headers.insert("X-API-Key".parse().unwrap(), api_key.parse().unwrap());
        }

        headers
    }

    // ==================== Authentication ====================

    pub async fn login(&mut self, username: &str, password: &str) -> Result<LoginResponse, Box<dyn std::error::Error>> {
        let url = format!("{}/auth/login", self.config.base_url);
        let request = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        let login_response: LoginResponse = response.json().await?;

        // Store access token
        self.config.access_token = Some(login_response.access_token.clone());

        Ok(login_response)
    }

    // ==================== Workflows ====================

    pub async fn create_workflow(&self, workflow: Workflow) -> Result<Workflow, Box<dyn std::error::Error>> {
        let url = format!("{}/workflows", self.config.base_url);

        let response = self.client
            .post(&url)
            .headers(self.get_auth_headers())
            .json(&workflow)
            .send()
            .await?;

        let created_workflow: Workflow = response.json().await?;
        Ok(created_workflow)
    }

    pub async fn list_workflows(&self, limit: u64, offset: u64) -> Result<WorkflowList, Box<dyn std::error::Error>> {
        let url = format!("{}/workflows?limit={}&offset={}", self.config.base_url, limit, offset);

        let response = self.client
            .get(&url)
            .headers(self.get_auth_headers())
            .send()
            .await?;

        let workflow_list: WorkflowList = response.json().await?;
        Ok(workflow_list)
    }

    pub async fn get_workflow(&self, workflow_id: &str) -> Result<Workflow, Box<dyn std::error::Error>> {
        let url = format!("{}/workflows/{}", self.config.base_url, workflow_id);

        let response = self.client
            .get(&url)
            .headers(self.get_auth_headers())
            .send()
            .await?;

        let workflow: Workflow = response.json().await?;
        Ok(workflow)
    }

    // ==================== Execution ====================

    pub async fn execute_workflow(
        &self,
        workflow_id: &str,
        inputs: HashMap<String, Value>,
        async_exec: bool,
    ) -> Result<ExecutionResponse, Box<dyn std::error::Error>> {
        let url = format!("{}/workflows/{}/execute", self.config.base_url, workflow_id);

        let request = ExecuteWorkflowRequest {
            inputs,
            async_exec,
            timeout_override: None,
        };

        let response = self.client
            .post(&url)
            .headers(self.get_auth_headers())
            .json(&request)
            .send()
            .await?;

        let execution: ExecutionResponse = response.json().await?;
        Ok(execution)
    }

    pub async fn get_execution_status(
        &self,
        workflow_id: &str,
        execution_id: &str,
    ) -> Result<ExecutionResponse, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/workflows/{}/status?executionId={}",
            self.config.base_url, workflow_id, execution_id
        );

        let response = self.client
            .get(&url)
            .headers(self.get_auth_headers())
            .send()
            .await?;

        let status: ExecutionResponse = response.json().await?;
        Ok(status)
    }

    pub async fn wait_for_completion(
        &self,
        workflow_id: &str,
        execution_id: &str,
        poll_interval_secs: u64,
        max_attempts: u32,
    ) -> Result<ExecutionResponse, Box<dyn std::error::Error>> {
        for _ in 0..max_attempts {
            let status = self.get_execution_status(workflow_id, execution_id).await?;

            if status.status == "completed" || status.status == "failed" {
                return Ok(status);
            }

            tokio::time::sleep(Duration::from_secs(poll_interval_secs)).await;
        }

        Err("Workflow execution timeout".into())
    }

    // ==================== Monitoring ====================

    pub async fn health_check(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let url = format!("{}/health", self.config.base_url);

        let response = self.client.get(&url).send().await?;
        let health: Value = response.json().await?;
        Ok(health)
    }
}

// ==================== Example Usage ====================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== LLM Orchestrator Rust Client Example ===\n");

    // Initialize client
    let config = ApiConfig::default();
    let mut client = LLMOrchestratorClient::new(config);

    // 1. Login
    println!("1. Logging in...");
    let login_response = client.login("admin", "secure_password").await?;
    println!("   Logged in as: {}", login_response.user_id);
    println!("   Roles: {:?}", login_response.roles);

    // 2. Create workflow
    println!("\n2. Creating workflow...");
    let workflow = Workflow {
        id: None,
        name: "sentiment-analyzer".to_string(),
        version: "1.0".to_string(),
        description: Some("Analyzes sentiment of input text".to_string()),
        steps: vec![serde_json::json!({
            "id": "analyze",
            "type": "llm",
            "provider": "openai",
            "model": "gpt-4",
            "prompt": "Analyze sentiment: {{input}}",
            "temperature": 0.3,
            "max_tokens": 50,
            "output": ["sentiment"]
        })],
        timeout_seconds: Some(300),
        metadata: None,
    };

    let created_workflow = client.create_workflow(workflow).await?;
    let workflow_id = created_workflow.id.as_ref().unwrap();
    println!("   Created workflow: {}", workflow_id);

    // 3. Execute workflow
    println!("\n3. Executing workflow...");
    let mut inputs = HashMap::new();
    inputs.insert("input".to_string(), serde_json::json!("This is amazing!"));

    let execution = client.execute_workflow(workflow_id, inputs, true).await?;
    println!("   Execution started: {}", execution.execution_id);

    // 4. Wait for completion
    println!("\n4. Waiting for completion...");
    let result = client
        .wait_for_completion(workflow_id, &execution.execution_id, 2, 60)
        .await?;
    println!("   Status: {}", result.status);

    // 5. List workflows
    println!("\n5. Listing workflows...");
    let workflows = client.list_workflows(5, 0).await?;
    println!("   Total workflows: {}", workflows.total);

    // 6. Health check
    println!("\n6. Health check...");
    let health = client.health_check().await?;
    println!("   Status: {}", health["status"]);

    println!("\n✓ Examples completed successfully!");

    Ok(())
}
