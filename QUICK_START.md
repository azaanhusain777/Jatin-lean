# Quick Start Guide — jatin-lean

> Get started in 60 seconds

---

## Installation

```bash
# Clone and build
git clone https://github.com/yourusername/jatin-lean.git
cd jatin-lean
cargo build --release

# Add to PATH (optional)
sudo cp target/release/jatin-lean /usr/local/bin/
```

---

## Basic Commands

```bash
# 1. See what would be deleted (dry run)
jatin-lean

# 2. Delete files with confirmation
jatin-lean --force

# 3. Delete files without confirmation (automation)
jatin-lean --force --yes

# 4. Show individual files
jatin-lean --verbose
```

---

## Configuration

```bash
# Generate config template
jatin-lean --init-config jatin-lean.toml

# Edit config
nano jatin-lean.toml

# Run (automatically uses jatin-lean.toml)
jatin-lean --force
```

**Example config:**
```toml
override_defaults = false
doc_files = ["CUSTOM_README.md"]
test_dirs = ["integration-tests"]
```

---

## Global Mode

```bash
# Scan all projects
jatin-lean ~/projects --global

# Limit depth
jatin-lean ~/projects --global --max-depth 3
```

---

## What Gets Deleted?

✅ **Safe to delete:**
- Documentation (README, CHANGELOG, etc.)
- Test files and directories
- CI/CD configs (.travis.yml, .circleci/)
- Examples and demos
- Source maps (*.js.map)
- Build artifacts (*.c, *.o, Makefile)

❌ **Never deleted:**
- `.bin/` directory
- Entry points (main, module, bin, exports)
- Type declarations (*.d.ts)
- Runtime dependencies

---

## Typical Savings

- **Small projects:** 10-30% reduction
- **Medium projects:** 30-50% reduction
- **Large projects:** 40-60% reduction

---

## Safety Features

1. **Dry run by default** — See changes before applying
2. **Entry point whitelisting** — Parses package.json
3. **Interactive confirmation** — Prompts before deletion
4. **Risk level indicators** — Shows safety assessment

---

## Common Use Cases

### CI/CD Pipeline
```bash
# In your CI script
jatin-lean --force --yes
```

### Docker Image Optimization
```dockerfile
RUN npm install && \
    npx jatin-lean --force --yes
```

### Pre-commit Hook
```bash
# .git/hooks/pre-commit
jatin-lean --verbose
```

### Monorepo Cleanup
```bash
jatin-lean ~/monorepo --global
```

---

## Troubleshooting

**Q: Nothing gets deleted**  
A: Your node_modules is already lean, or files are whitelisted as runtime-required.

**Q: Can I undo deletion?**  
A: Not yet. Always run dry-run first to verify.

**Q: How do I keep LICENSE files?**  
A: Create a config with `doc_files = []` and `override_defaults = true`.

**Q: Does it work on Windows?**  
A: Yes, but locked file handling is basic.

---

## Help

```bash
jatin-lean --help
```

---

**Made with ❤️ by Jatin Jalandhra**
