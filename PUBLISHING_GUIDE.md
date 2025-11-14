# Publishing LLM Orchestrator Crates to crates.io

This guide provides step-by-step instructions for publishing all 8 crates to crates.io.

---

## Prerequisites

### 1. Create crates.io Account
- Go to https://crates.io
- Sign in with GitHub
- Verify your email address

### 2. Generate API Token
1. Go to https://crates.io/settings/tokens
2. Click "New Token"
3. Name: "llm-orchestrator-publishing"
4. Scopes: Select "publish-new" and "publish-update"
5. Click "Generate token"
6. **Copy the token immediately** (it won't be shown again)

### 3. Configure Cargo with Your Token

```bash
# Method 1: Using cargo login (recommended)
cargo login YOUR_API_TOKEN_HERE

# Method 2: Manual configuration
# Add to ~/.cargo/credentials.toml:
# [registry]
# token = "YOUR_API_TOKEN_HERE"
```

**SECURITY WARNING**: Never commit your API token to git!

---

## Pre-Publishing Checklist

Before publishing, ensure:

- [x] All crates build successfully: `cargo build --all --release`
- [x] All tests pass: `cargo test --all`
- [x] Zero compilation warnings
- [x] All Cargo.toml files have correct metadata:
  - version = "0.1.0"
  - authors
  - license = "MIT OR Apache-2.0"
  - description
  - repository
  - keywords
  - categories
- [x] README.md exists for each crate
- [x] Documentation is complete: `cargo doc --all --no-deps`

---

## Publishing Order (IMPORTANT!)

Crates must be published in dependency order. Dependent crates cannot be published until their dependencies are live on crates.io.

### Dependency Order:

```
1. llm-orchestrator-providers (no dependencies)
2. llm-orchestrator-state (no core dependencies)
3. llm-orchestrator-auth (no core dependencies)
4. llm-orchestrator-secrets (no core dependencies)
5. llm-orchestrator-audit (no core dependencies)
6. llm-orchestrator-core (depends on providers)
7. llm-orchestrator-sdk (depends on core)
8. llm-orchestrator-cli (depends on core)
```

---

## Publishing Commands

### Set Your API Token First

```bash
# Replace YOUR_TOKEN with your actual crates.io API token
export CARGO_REGISTRY_TOKEN="YOUR_TOKEN"

# OR use cargo login
cargo login YOUR_TOKEN
```

### Publish Each Crate in Order

**IMPORTANT**: Wait 2-3 minutes between publishing dependent crates to allow crates.io to index them.

```bash
# 1. Publish providers (no dependencies)
cd crates/llm-orchestrator-providers
cargo publish --allow-dirty
cd ../..

# Wait 2-3 minutes for indexing...
echo "Waiting for crates.io to index llm-orchestrator-providers..."
sleep 180

# 2. Publish state (independent)
cd crates/llm-orchestrator-state
cargo publish --allow-dirty
cd ../..

# 3. Publish auth (independent)
cd crates/llm-orchestrator-auth
cargo publish --allow-dirty
cd ../..

# 4. Publish secrets (independent)
cd crates/llm-orchestrator-secrets
cargo publish --allow-dirty
cd ../..

# 5. Publish audit (independent)
cd crates/llm-orchestrator-audit
cargo publish --allow-dirty
cd ../..

# Wait 2-3 minutes for indexing...
echo "Waiting for crates.io to index independent crates..."
sleep 180

# 6. Publish core (depends on providers)
cd crates/llm-orchestrator-core
cargo publish --allow-dirty
cd ../..

# Wait 2-3 minutes for indexing...
echo "Waiting for crates.io to index llm-orchestrator-core..."
sleep 180

# 7. Publish SDK (depends on core)
cd crates/llm-orchestrator-sdk
cargo publish --allow-dirty
cd ../..

# 8. Publish CLI (depends on core)
cd crates/llm-orchestrator-cli
cargo publish --allow-dirty
cd ../..
```

---

## Automated Publishing Script

Use the provided script for automated publishing:

```bash
# Make the script executable
chmod +x scripts/publish-all-crates.sh

# Set your API token
export CARGO_REGISTRY_TOKEN="YOUR_TOKEN"

# Run the script
./scripts/publish-all-crates.sh
```

---

## Dry Run (Test Publishing)

Before actual publishing, do a dry run to catch errors:

```bash
# Test publish without actually uploading
cargo publish --dry-run -p llm-orchestrator-providers
cargo publish --dry-run -p llm-orchestrator-state
cargo publish --dry-run -p llm-orchestrator-auth
cargo publish --dry-run -p llm-orchestrator-secrets
cargo publish --dry-run -p llm-orchestrator-audit
cargo publish --dry-run -p llm-orchestrator-core
cargo publish --dry-run -p llm-orchestrator-sdk
cargo publish --dry-run -p llm-orchestrator-cli
```

---

## Verifying Publication

After publishing, verify each crate appears on crates.io:

1. **llm-orchestrator-providers**: https://crates.io/crates/llm-orchestrator-providers
2. **llm-orchestrator-state**: https://crates.io/crates/llm-orchestrator-state
3. **llm-orchestrator-auth**: https://crates.io/crates/llm-orchestrator-auth
4. **llm-orchestrator-secrets**: https://crates.io/crates/llm-orchestrator-secrets
5. **llm-orchestrator-audit**: https://crates.io/crates/llm-orchestrator-audit
6. **llm-orchestrator-core**: https://crates.io/crates/llm-orchestrator-core
7. **llm-orchestrator-sdk**: https://crates.io/crates/llm-orchestrator-sdk
8. **llm-orchestrator-cli**: https://crates.io/crates/llm-orchestrator-cli

---

## Common Issues and Solutions

### Issue: "crate name already exists"
**Solution**: The crate name is taken. You'll need to:
1. Choose a different name (e.g., add your org prefix)
2. Update name in Cargo.toml
3. Update all dependencies referencing that crate

### Issue: "failed to verify package tarball"
**Solution**: Run `cargo package` first to identify missing files, then add them to the package.

### Issue: "missing required field: description"
**Solution**: Add description to Cargo.toml:
```toml
description = "Your crate description"
```

### Issue: "authentication required"
**Solution**: Run `cargo login YOUR_TOKEN` again.

### Issue: "dependency not found on crates.io"
**Solution**: Wait longer for crates.io to index, or publish dependencies first.

---

## Post-Publishing Checklist

After all crates are published:

- [ ] Verify all crates appear on crates.io
- [ ] Test installation: `cargo install llm-orchestrator-cli`
- [ ] Update main README.md with crates.io badges
- [ ] Create GitHub release tag (v0.1.0)
- [ ] Announce release (blog, Twitter, Reddit /r/rust)
- [ ] Update documentation site

---

## Version Updates (Future Releases)

For subsequent releases:

1. Update version in workspace Cargo.toml:
   ```toml
   [workspace.package]
   version = "0.2.0"
   ```

2. Update CHANGELOG.md

3. Commit and tag:
   ```bash
   git add -A
   git commit -m "chore: bump version to 0.2.0"
   git tag v0.2.0
   git push origin main --tags
   ```

4. Publish crates in dependency order (same as above)

---

## Security Notes

1. **Never commit credentials**: Add `.cargo/credentials.toml` to `.gitignore`
2. **Rotate tokens regularly**: Generate new tokens every 90 days
3. **Use scoped tokens**: Only grant necessary permissions
4. **Revoke compromised tokens**: Immediately revoke if exposed

---

## Getting Help

- **Cargo Book**: https://doc.rust-lang.org/cargo/reference/publishing.html
- **crates.io Guide**: https://doc.rust-lang.org/cargo/reference/publishing.html
- **Rust Community**: https://users.rust-lang.org/

---

**Last Updated**: 2025-11-14
**Version**: 0.1.0
