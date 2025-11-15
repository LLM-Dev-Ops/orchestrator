# @llm-dev-ops/llm-orchestrator-linux-arm64

[![npm version](https://img.shields.io/npm/v/@llm-dev-ops/llm-orchestrator-linux-arm64.svg)](https://www.npmjs.com/package/@llm-dev-ops/llm-orchestrator-linux-arm64)

This package contains the **native Linux ARM64 binary** for LLM Orchestrator.

## 🔍 What is this?

This is a platform-specific package that contains the pre-compiled binary for **Linux ARM64 (aarch64)** systems. It is automatically installed as an optional dependency when you install the main `@llm-dev-ops/llm-orchestrator` package on a compatible system.

## 📦 Installation

**You typically don't need to install this package directly.** Instead, install the main package:

```bash
npm install -g @llm-dev-ops/llm-orchestrator
```

The correct platform-specific binary will be automatically selected based on your system.

## ⚙️ Platform Support

- **Operating System**: Linux
- **Architecture**: ARM64 (aarch64)
- **Minimum glibc**: 2.31
- **Ideal For**: Raspberry Pi 4+, AWS Graviton, Oracle Ampere

## 📚 Documentation

For usage instructions and complete documentation, see:

- **Main Package**: [@llm-dev-ops/llm-orchestrator](https://www.npmjs.com/package/@llm-dev-ops/llm-orchestrator)
- **GitHub**: [llm-orchestrator](https://github.com/globalbusinessadvisors/llm-orchestrator)

## 📄 License

MIT OR Apache-2.0
