# Vector Database Integration Guide

Quick reference for using Pinecone, Weaviate, and Qdrant vector databases with LLM Orchestrator.

---

## Quick Start

### Installation

Add to your `Cargo.toml`:
```toml
[dependencies]
llm-orchestrator-providers = "0.1.0"
```

### Basic Usage

```rust
use llm_orchestrator_providers::{
    PineconeClient, WeaviateClient, QdrantClient,
    VectorSearchProvider, VectorSearchRequest,
    UpsertRequest, VectorRecord, DeleteRequest,
};
```

---

## Pinecone

### Setup
```rust
let client = PineconeClient::new(
    std::env::var("PINECONE_API_KEY")?,
    "us-west1-gcp".to_string(),  // Your environment
)?;
```

### Search
```rust
let request = VectorSearchRequest {
    index: "my-index".to_string(),
    query: vec![0.1, 0.2, 0.3],
    top_k: 10,
    namespace: Some("production".to_string()),
    filter: Some(json!({"category": "docs"})),
    include_metadata: true,
    include_vectors: false,
};

let response = client.search(request).await?;
```

### Upsert
```rust
let request = UpsertRequest {
    index: "my-index".to_string(),
    vectors: vec![
        VectorRecord {
            id: "doc1".to_string(),
            vector: vec![0.1, 0.2, 0.3],
            metadata: Some(json!({"text": "Example"})),
        },
    ],
    namespace: Some("production".to_string()),
};

let response = client.upsert(request).await?;
```

### Delete
```rust
let request = DeleteRequest {
    index: "my-index".to_string(),
    ids: vec!["doc1".to_string()],
    namespace: Some("production".to_string()),
    delete_all: false,
};

let response = client.delete(request).await?;
```

---

## Weaviate

### Setup
```rust
let client = WeaviateClient::new(
    "http://localhost:8080".to_string(),
    Some(std::env::var("WEAVIATE_API_KEY")?),  // Optional
)?;
```

### Search
```rust
let request = VectorSearchRequest {
    index: "Article".to_string(),  // Class name
    query: vec![0.1, 0.2, 0.3],
    top_k: 5,
    namespace: None,
    filter: None,
    include_metadata: true,
    include_vectors: false,
};

let response = client.search(request).await?;
```

### Upsert
```rust
let request = UpsertRequest {
    index: "Article".to_string(),
    vectors: vec![
        VectorRecord {
            id: uuid::Uuid::new_v4().to_string(),
            vector: vec![0.1, 0.2, 0.3],
            metadata: Some(json!({
                "title": "My Article",
                "content": "Article content..."
            })),
        },
    ],
    namespace: None,
};

let response = client.upsert(request).await?;
```

### Delete
```rust
let request = DeleteRequest {
    index: "Article".to_string(),
    ids: vec![
        "uuid-1".to_string(),
        "uuid-2".to_string(),
    ],
    namespace: None,
    delete_all: false,
};

let response = client.delete(request).await?;
```

---

## Qdrant

### Setup
```rust
let client = QdrantClient::new(
    "http://localhost:6333".to_string(),
    Some(std::env::var("QDRANT_API_KEY")?),  // Optional
)?;
```

### Search
```rust
let request = VectorSearchRequest {
    index: "my-collection".to_string(),
    query: vec![0.1, 0.2, 0.3],
    top_k: 10,
    namespace: None,
    filter: Some(json!({
        "must": [
            {"key": "category", "match": {"value": "documentation"}}
        ]
    })),
    include_metadata: true,
    include_vectors: false,
};

let response = client.search(request).await?;
```

### Upsert
```rust
let request = UpsertRequest {
    index: "my-collection".to_string(),
    vectors: vec![
        VectorRecord {
            id: "point1".to_string(),
            vector: vec![0.1, 0.2, 0.3],
            metadata: Some(json!({"text": "Example"})),
        },
    ],
    namespace: None,
};

let response = client.upsert(request).await?;
```

### Delete
```rust
let request = DeleteRequest {
    index: "my-collection".to_string(),
    ids: vec!["point1".to_string(), "point2".to_string()],
    namespace: None,
    delete_all: false,
};

let response = client.delete(request).await?;
```

---

## Comparison

| Feature | Pinecone | Weaviate | Qdrant |
|---------|----------|----------|--------|
| **Deployment** | Cloud only | Cloud or self-hosted | Cloud or self-hosted |
| **API Style** | REST | REST + GraphQL | REST |
| **Authentication** | API key | Optional Bearer token | Optional API key |
| **Namespaces** | ✅ Yes | ❌ No (use classes) | ❌ No (use collections) |
| **Metadata Filtering** | ✅ Yes | ✅ Yes | ✅ Yes |
| **Batch Operations** | ✅ Yes (1000/batch) | ✅ Yes | ✅ Yes |
| **ID Types** | String | UUID | UUID or Integer |
| **Best For** | Production, managed | Flexible schema | Self-hosted, fast |

---

## Error Handling

```rust
use llm_orchestrator_providers::ProviderError;

match client.search(request).await {
    Ok(response) => {
        println!("Found {} results", response.results.len());
    }
    Err(ProviderError::AuthError(msg)) => {
        eprintln!("Authentication failed: {}", msg);
    }
    Err(ProviderError::RateLimitExceeded) => {
        eprintln!("Rate limit exceeded, retry later");
    }
    Err(ProviderError::InvalidRequest(msg)) => {
        eprintln!("Invalid request: {}", msg);
    }
    Err(e) => {
        eprintln!("Unexpected error: {}", e);
    }
}
```

---

## RAG Workflow Example

```yaml
name: "RAG Question Answering"
version: "1.0"
steps:
  - id: embed_query
    type: embed
    provider: openai
    model: text-embedding-3-small
    input: "{{inputs.question}}"
    output: [query_vector]

  - id: search_docs
    type: vector_search
    depends_on: [embed_query]
    database: qdrant
    index: knowledge_base
    query: "{{outputs.query_vector}}"
    top_k: 5
    filter:
      must:
        - key: "category"
          match:
            value: "documentation"
    include_metadata: true
    output: [relevant_docs]

  - id: generate_answer
    type: llm
    depends_on: [search_docs]
    provider: anthropic
    model: claude-3-5-sonnet-20241022
    prompt: |
      Use the following context to answer the question.

      Context:
      {{#each outputs.relevant_docs}}
      - {{this.metadata.text}} (relevance: {{this.score}})
      {{/each}}

      Question: {{inputs.question}}

      Answer:
    output: [answer]
```

Execute:
```rust
let workflow = Workflow::from_yaml(&yaml_content)?;
let executor = WorkflowExecutor::new(workflow)
    .with_vector_db("qdrant", qdrant_client)
    .with_embedding_provider("openai", openai_embeddings)
    .with_llm_provider("anthropic", anthropic_client);

let result = executor.execute(json!({
    "question": "What is the difference between Pinecone and Qdrant?"
})).await?;

println!("Answer: {}", result.outputs["answer"]);
```

---

## Performance Tips

1. **Batch Upserts:** Insert multiple vectors in a single request
   ```rust
   // Good: 100 vectors in 1 request
   let vectors = (0..100).map(|i| VectorRecord { ... }).collect();
   client.upsert(UpsertRequest { vectors, ... }).await?;

   // Bad: 100 requests
   for i in 0..100 {
       client.upsert(UpsertRequest { vectors: vec![...], ... }).await?;
   }
   ```

2. **Connection Reuse:** Create one client, use for multiple operations
   ```rust
   let client = PineconeClient::new(...)?;

   for query in queries {
       client.search(query).await?;  // Reuses connection
   }
   ```

3. **Selective Metadata:** Only request metadata when needed
   ```rust
   VectorSearchRequest {
       include_metadata: true,   // Only if you need it
       include_vectors: false,   // Usually not needed for search
       ...
   }
   ```

4. **Filtering:** Push filters to vector DB instead of filtering in code
   ```rust
   // Good: Filter at DB level
   VectorSearchRequest {
       filter: Some(json!({"category": "tech"})),
       top_k: 10,
       ...
   }

   // Bad: Fetch 100, filter in code
   let all_results = client.search(VectorSearchRequest { top_k: 100, ... }).await?;
   let filtered: Vec<_> = all_results.results.into_iter()
       .filter(|r| r.metadata.get("category") == Some("tech"))
       .take(10)
       .collect();
   ```

---

## Testing

Mock vector database for testing:
```rust
use mockito::Server;
use llm_orchestrator_providers::{QdrantClient, VectorSearchProvider};

#[tokio::test]
async fn test_vector_search() {
    let mut server = Server::new_async().await;

    let mock = server.mock("POST", "/collections/test/points/search")
        .with_status(200)
        .with_body(r#"{
            "result": [
                {
                    "id": "1",
                    "score": 0.95,
                    "payload": {"text": "test"}
                }
            ],
            "status": "ok"
        }"#)
        .create_async()
        .await;

    let client = QdrantClient::new(server.url(), None).unwrap();

    let response = client.search(VectorSearchRequest {
        index: "test".to_string(),
        query: vec![0.1, 0.2, 0.3],
        top_k: 1,
        namespace: None,
        filter: None,
        include_metadata: true,
        include_vectors: false,
    }).await.unwrap();

    assert_eq!(response.results.len(), 1);
    assert_eq!(response.results[0].id, "1");

    mock.assert_async().await;
}
```

---

## Resources

- **Pinecone:** https://docs.pinecone.io/reference
- **Weaviate:** https://weaviate.io/developers/weaviate/api/rest
- **Qdrant:** https://qdrant.tech/documentation/

---

**Last Updated:** 2025-11-14
