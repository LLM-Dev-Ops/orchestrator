# LLM Orchestrator

<div align="center">

[![npm version](https://img.shields.io/npm/v/@llm-dev-ops/llm-orchestrator.svg)](https://www.npmjs.com/package/@llm-dev-ops/llm-orchestrator)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/globalbusinessadvisors/llm-orchestrator)
[![Downloads](https://img.shields.io/npm/dm/@llm-dev-ops/llm-orchestrator.svg)](https://www.npmjs.com/package/@llm-dev-ops/llm-orchestrator)

**Production-ready LLM workflow orchestrator with DAG execution, state management, and multi-provider support**

[Features](#-features) • [Installation](#-installation) • [Quick Start](#-quick-start) • [Documentation](#-documentation) • [Examples](#-examples)

</div>

---

## 🚀 Features

- **🔄 DAG-Based Execution** - Define complex workflows with dependencies and parallel execution
- **🎯 Multi-Provider Support** - Seamlessly work with OpenAI, Anthropic Claude, and more
- **💾 State Management** - Persistent state across workflow runs with built-in caching
- **📝 Template Engine** - Handlebars-based templating for dynamic prompt generation
- **📊 Observability** - Built-in metrics, tracing, and comprehensive logging
- **🔒 Type Safety** - Full Rust implementation for reliability and performance
- **⚡ High Performance** - Concurrent execution with configurable parallelism
- **🛡️ Error Handling** - Automatic retry policies and graceful error recovery

## 📦 Installation

### Global Installation (Recommended for CLI)

```bash
npm install -g @llm-dev-ops/llm-orchestrator
```

### Project Installation

```bash
npm install @llm-dev-ops/llm-orchestrator
```

### Docker

```bash
docker pull ghcr.io/globalbusinessadvisors/llm-orchestrator:latest
```

## 🎯 Quick Start

### CLI Usage

Create a workflow file `workflow.yaml`:

```yaml
name: sentiment-analysis
version: "1.0"

providers:
  openai:
    type: openai
    model: gpt-4

steps:
  - id: analyze
    provider: openai
    prompt: "Analyze the sentiment of: {{input.text}}"

  - id: summarize
    provider: openai
    prompt: "Summarize this sentiment analysis: {{steps.analyze.output}}"
    depends_on: [analyze]
```

Run the workflow:

```bash
llm-orchestrator run workflow.yaml --input '{"text": "I love this product!"}'
```

### Programmatic Usage

```javascript
const orchestrator = require('@llm-dev-ops/llm-orchestrator');

// Run a workflow
const result = await orchestrator.run('workflow.yaml', {
  input: JSON.stringify({ text: 'I love this product!' }),
  maxConcurrency: 5
});

console.log('Result:', result.stdout);
```

## 📚 Examples

### Multi-Step Content Generation

```yaml
name: content-pipeline
version: "1.0"

providers:
  claude:
    type: anthropic
    model: claude-3-5-sonnet-20241022

steps:
  - id: outline
    provider: claude
    prompt: "Create a blog post outline about {{input.topic}}"

  - id: draft
    provider: claude
    prompt: "Write a blog post based on this outline: {{steps.outline.output}}"
    depends_on: [outline]

  - id: edit
    provider: claude
    prompt: "Edit and improve this draft: {{steps.draft.output}}"
    depends_on: [draft]
```

### Parallel Processing

```yaml
name: parallel-analysis
version: "1.0"

providers:
  openai:
    type: openai
    model: gpt-4

steps:
  - id: sentiment
    provider: openai
    prompt: "Analyze sentiment: {{input.text}}"

  - id: entities
    provider: openai
    prompt: "Extract entities from: {{input.text}}"

  - id: summary
    provider: openai
    prompt: "Summarize sentiment={{steps.sentiment.output}} entities={{steps.entities.output}}"
    depends_on: [sentiment, entities]
```

## 🔧 CLI Commands

```bash
# Validate a workflow
llm-orchestrator validate workflow.yaml

# Run a workflow
llm-orchestrator run workflow.yaml

# Run with input
llm-orchestrator run workflow.yaml --input '{"key": "value"}'

# Set concurrency limit
llm-orchestrator run workflow.yaml --max-concurrency 10

# Show version
llm-orchestrator --version
```

## 🌍 Supported Platforms

| Platform | Architecture | Status |
|----------|-------------|---------|
| Linux | x64 | ✅ Supported |
| Linux | ARM64 | ✅ Supported |
| macOS | Intel (x64) | ✅ Supported |
| macOS | Apple Silicon (ARM64) | ✅ Supported |

## 🔑 Environment Variables

```bash
# OpenAI
export OPENAI_API_KEY="sk-..."

# Anthropic
export ANTHROPIC_API_KEY="sk-ant-..."

# Custom endpoint (optional)
export LLM_ORCHESTRATOR_ENDPOINT="https://api.example.com"
```

## 📖 Documentation

For comprehensive documentation, visit:
- **GitHub Repository**: https://github.com/globalbusinessadvisors/llm-orchestrator
- **API Reference**: https://docs.rs/llm-orchestrator
- **Examples**: https://github.com/globalbusinessadvisors/llm-orchestrator/tree/main/examples

## 🤝 Contributing

Contributions are welcome! Please see our [Contributing Guide](https://github.com/globalbusinessadvisors/llm-orchestrator/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under either of:

- MIT License ([LICENSE-MIT](https://opensource.org/licenses/MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](https://www.apache.org/licenses/LICENSE-2.0))

at your option.

## 💬 Support

- **Issues**: [GitHub Issues](https://github.com/globalbusinessadvisors/llm-orchestrator/issues)
- **Discussions**: [GitHub Discussions](https://github.com/globalbusinessadvisors/llm-orchestrator/discussions)

---

<div align="center">

Made with ❤️ by the LLM DevOps Team

</div>
