"""
LLM Orchestrator Python Client

Example usage of the LLM Orchestrator API with Python.
"""

import requests
import json
import time
from typing import Dict, List, Optional, Any
from dataclasses import dataclass
from datetime import datetime


@dataclass
class ApiConfig:
    """API configuration"""
    base_url: str = "https://api.llm-orchestrator.io/api/v1"
    access_token: Optional[str] = None
    api_key: Optional[str] = None


class LLMOrchestratorClient:
    """Client for LLM Orchestrator API"""

    def __init__(self, config: ApiConfig):
        self.config = config
        self.session = requests.Session()
        self._update_auth_headers()

    def _update_auth_headers(self):
        """Update authentication headers"""
        if self.config.access_token:
            self.session.headers.update({
                "Authorization": f"Bearer {self.config.access_token}"
            })
        elif self.config.api_key:
            self.session.headers.update({
                "X-API-Key": self.config.api_key
            })

    def _request(
        self,
        method: str,
        endpoint: str,
        data: Optional[Dict] = None,
        params: Optional[Dict] = None
    ) -> Dict:
        """Make HTTP request"""
        url = f"{self.config.base_url}{endpoint}"
        response = self.session.request(
            method=method,
            url=url,
            json=data,
            params=params
        )
        response.raise_for_status()

        # Return empty dict for 204 No Content
        if response.status_code == 204:
            return {}

        return response.json()

    # ==================== Authentication ====================

    def login(self, username: str, password: str) -> Dict:
        """Login with username and password"""
        response = requests.post(
            f"{self.config.base_url}/auth/login",
            json={"username": username, "password": password}
        )
        response.raise_for_status()
        data = response.json()

        # Store access token
        self.config.access_token = data["access_token"]
        self._update_auth_headers()

        return data

    def refresh_token(self, refresh_token: str) -> Dict:
        """Refresh JWT access token"""
        response = self._request(
            "POST",
            "/auth/refresh",
            data={"refresh_token": refresh_token}
        )

        # Update stored token
        self.config.access_token = response["access_token"]
        self._update_auth_headers()

        return response

    def create_api_key(
        self,
        name: str,
        scopes: List[str],
        expires_in_days: int = 90
    ) -> Dict:
        """Create a new API key"""
        return self._request(
            "POST",
            "/auth/keys",
            data={
                "name": name,
                "scopes": scopes,
                "expires_in_days": expires_in_days
            }
        )

    def list_api_keys(self, limit: int = 20, offset: int = 0) -> Dict:
        """List all API keys"""
        return self._request(
            "GET",
            "/auth/keys",
            params={"limit": limit, "offset": offset}
        )

    def revoke_api_key(self, key_id: str) -> None:
        """Revoke an API key"""
        self._request("DELETE", f"/auth/keys/{key_id}")

    # ==================== Workflows ====================

    def create_workflow(self, workflow: Dict) -> Dict:
        """Create a new workflow"""
        return self._request("POST", "/workflows", data=workflow)

    def list_workflows(
        self,
        limit: int = 20,
        offset: int = 0,
        name: Optional[str] = None,
        version: Optional[str] = None
    ) -> Dict:
        """List workflows with optional filtering"""
        params = {"limit": limit, "offset": offset}
        if name:
            params["name"] = name
        if version:
            params["version"] = version

        return self._request("GET", "/workflows", params=params)

    def get_workflow(self, workflow_id: str) -> Dict:
        """Get workflow details"""
        return self._request("GET", f"/workflows/{workflow_id}")

    def update_workflow(self, workflow_id: str, workflow: Dict) -> Dict:
        """Update workflow definition"""
        return self._request("PUT", f"/workflows/{workflow_id}", data=workflow)

    def delete_workflow(self, workflow_id: str) -> None:
        """Delete a workflow"""
        self._request("DELETE", f"/workflows/{workflow_id}")

    # ==================== Execution ====================

    def execute_workflow(
        self,
        workflow_id: str,
        inputs: Dict,
        async_execution: bool = True,
        timeout_override: Optional[int] = None
    ) -> Dict:
        """Execute a workflow"""
        data = {
            "inputs": inputs,
            "async": async_execution
        }
        if timeout_override:
            data["timeout_override"] = timeout_override

        return self._request(
            "POST",
            f"/workflows/{workflow_id}/execute",
            data=data
        )

    def get_execution_status(
        self,
        workflow_id: str,
        execution_id: str
    ) -> Dict:
        """Get execution status"""
        return self._request(
            "GET",
            f"/workflows/{workflow_id}/status",
            params={"executionId": execution_id}
        )

    def pause_workflow(self, workflow_id: str, execution_id: str) -> Dict:
        """Pause workflow execution"""
        return self._request(
            "POST",
            f"/workflows/{workflow_id}/pause",
            params={"executionId": execution_id}
        )

    def resume_workflow(self, workflow_id: str, execution_id: str) -> Dict:
        """Resume paused workflow"""
        return self._request(
            "POST",
            f"/workflows/{workflow_id}/resume",
            params={"executionId": execution_id}
        )

    def cancel_workflow(self, workflow_id: str, execution_id: str) -> Dict:
        """Cancel workflow execution"""
        return self._request(
            "POST",
            f"/workflows/{workflow_id}/cancel",
            params={"executionId": execution_id}
        )

    def wait_for_completion(
        self,
        workflow_id: str,
        execution_id: str,
        poll_interval: int = 2,
        max_attempts: int = 60
    ) -> Dict:
        """
        Poll execution status until completion

        Args:
            workflow_id: Workflow ID
            execution_id: Execution ID
            poll_interval: Seconds between polls
            max_attempts: Maximum polling attempts

        Returns:
            Final execution state

        Raises:
            TimeoutError: If max attempts exceeded
        """
        for attempt in range(max_attempts):
            status = self.get_execution_status(workflow_id, execution_id)

            if status["status"] in ["completed", "failed"]:
                return status

            time.sleep(poll_interval)

        raise TimeoutError(
            f"Workflow execution did not complete within "
            f"{max_attempts * poll_interval} seconds"
        )

    # ==================== State Management ====================

    def get_workflow_state(self, workflow_id: str, execution_id: str) -> Dict:
        """Get workflow state"""
        return self._request(
            "GET",
            f"/state/{workflow_id}",
            params={"executionId": execution_id}
        )

    def list_checkpoints(
        self,
        workflow_id: str,
        execution_id: str,
        limit: int = 20,
        offset: int = 0
    ) -> Dict:
        """List checkpoints"""
        return self._request(
            "GET",
            f"/state/{workflow_id}/checkpoints",
            params={
                "executionId": execution_id,
                "limit": limit,
                "offset": offset
            }
        )

    def create_checkpoint(
        self,
        workflow_id: str,
        execution_id: str,
        step_id: str
    ) -> Dict:
        """Create a checkpoint"""
        return self._request(
            "POST",
            f"/state/{workflow_id}/checkpoints",
            data={
                "executionId": execution_id,
                "stepId": step_id
            }
        )

    def restore_checkpoint(self, workflow_id: str, checkpoint_id: str) -> Dict:
        """Restore from checkpoint"""
        return self._request(
            "POST",
            f"/state/{workflow_id}/restore",
            data={"checkpointId": checkpoint_id}
        )

    # ==================== Monitoring ====================

    def health_check(self) -> Dict:
        """Get health status"""
        response = requests.get(f"{self.config.base_url}/health")
        response.raise_for_status()
        return response.json()

    def readiness_probe(self) -> bool:
        """Check if service is ready"""
        try:
            response = requests.get(f"{self.config.base_url}/health/ready")
            return response.status_code == 200
        except Exception:
            return False

    def liveness_probe(self) -> bool:
        """Check if service is alive"""
        try:
            response = requests.get(f"{self.config.base_url}/health/live")
            return response.status_code == 200
        except Exception:
            return False

    def get_metrics(self) -> str:
        """Get Prometheus metrics"""
        response = requests.get(f"{self.config.base_url}/metrics")
        response.raise_for_status()
        return response.text

    # ==================== Audit ====================

    def query_audit_events(
        self,
        user_id: Optional[str] = None,
        event_type: Optional[str] = None,
        resource_type: Optional[str] = None,
        resource_id: Optional[str] = None,
        start_time: Optional[datetime] = None,
        end_time: Optional[datetime] = None,
        result: Optional[str] = None,
        limit: int = 20,
        offset: int = 0
    ) -> Dict:
        """Query audit events"""
        params = {"limit": limit, "offset": offset}

        if user_id:
            params["user_id"] = user_id
        if event_type:
            params["event_type"] = event_type
        if resource_type:
            params["resource_type"] = resource_type
        if resource_id:
            params["resource_id"] = resource_id
        if start_time:
            params["start_time"] = start_time.isoformat()
        if end_time:
            params["end_time"] = end_time.isoformat()
        if result:
            params["result"] = result

        return self._request("GET", "/audit/events", params=params)

    def get_audit_event(self, event_id: str) -> Dict:
        """Get audit event details"""
        return self._request("GET", f"/audit/events/{event_id}")

    # ==================== Admin ====================

    def list_users(self, limit: int = 20, offset: int = 0) -> Dict:
        """List all users (admin only)"""
        return self._request(
            "GET",
            "/admin/users",
            params={"limit": limit, "offset": offset}
        )

    def create_user(
        self,
        username: str,
        email: str,
        password: str,
        roles: List[str]
    ) -> Dict:
        """Create a new user (admin only)"""
        return self._request(
            "POST",
            "/admin/users",
            data={
                "username": username,
                "email": email,
                "password": password,
                "roles": roles
            }
        )

    def get_system_stats(self) -> Dict:
        """Get system statistics (admin only)"""
        return self._request("GET", "/admin/stats")

    def manage_secret(
        self,
        key: str,
        value: str,
        ttl_seconds: Optional[int] = None
    ) -> Dict:
        """Store or update a secret (admin only)"""
        data = {"key": key, "value": value}
        if ttl_seconds:
            data["ttl_seconds"] = ttl_seconds

        return self._request("POST", "/admin/secrets", data=data)

    def get_system_config(self) -> Dict:
        """Get system configuration (admin only)"""
        return self._request("GET", "/admin/config")


# ==================== Example Usage ====================

def main():
    """Example usage"""

    # Initialize client
    config = ApiConfig(base_url="https://api.llm-orchestrator.io/api/v1")
    client = LLMOrchestratorClient(config)

    # 1. Login
    print("1. Logging in...")
    login_response = client.login("admin", "secure_password")
    print(f"   Logged in as: {login_response['user_id']}")
    print(f"   Roles: {login_response['roles']}")

    # 2. Create workflow
    print("\n2. Creating workflow...")
    workflow = {
        "name": "sentiment-analyzer",
        "version": "1.0",
        "description": "Analyzes sentiment of input text",
        "steps": [
            {
                "id": "analyze",
                "type": "llm",
                "provider": "openai",
                "model": "gpt-4",
                "prompt": "Analyze sentiment: {{input}}",
                "temperature": 0.3,
                "max_tokens": 50,
                "output": ["sentiment"]
            }
        ]
    }
    workflow_response = client.create_workflow(workflow)
    workflow_id = workflow_response["id"]
    print(f"   Created workflow: {workflow_id}")

    # 3. Execute workflow
    print("\n3. Executing workflow...")
    execution = client.execute_workflow(
        workflow_id=workflow_id,
        inputs={"input": "This is amazing!"}
    )
    execution_id = execution["execution_id"]
    print(f"   Execution started: {execution_id}")

    # 4. Wait for completion
    print("\n4. Waiting for completion...")
    result = client.wait_for_completion(workflow_id, execution_id)
    print(f"   Status: {result['status']}")
    if result['status'] == 'completed':
        print(f"   Outputs: {result.get('context', {}).get('outputs', {})}")

    # 5. List workflows
    print("\n5. Listing workflows...")
    workflows = client.list_workflows(limit=5)
    print(f"   Total workflows: {workflows['total']}")

    # 6. Query audit events
    print("\n6. Querying audit events...")
    events = client.query_audit_events(
        event_type="workflow_execution",
        limit=5
    )
    print(f"   Total audit events: {events['total']}")

    # 7. Health check
    print("\n7. Health check...")
    health = client.health_check()
    print(f"   Status: {health['status']}")

    print("\n✓ Examples completed successfully!")


if __name__ == "__main__":
    main()
