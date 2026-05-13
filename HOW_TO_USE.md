# 📖 How to Use jatin-lean - Complete User Guide

**Created by Jatin Jalandhra**

A comprehensive guide for using jatin-lean to optimize your Node.js projects.

---

## 🚀 Quick Start

### Installation

**Option 1: Use with npx (No Installation - Recommended)**
```bash
npx jatin-lean
```

**Option 2: Global Installation**
```bash
npm install -g jatin-lean
jatin-lean
```

**Option 3: Project Dependency**
```bash
npm install --save-dev jatin-lean
npx jatin-lean
```

---

## 🎯 Basic Usage

### 1. Dry Run (Safe Preview)

**Always start with a dry run** to see what would be deleted:

```bash
# In your project directory
cd /path/to/your/project
npx jatin-lean
```

**What you'll see:**
```
  ╔═══════════════════════════════════════════════╗
  ║  ⚡ jatin-lean — Node Modules Pruner ⚡      ║
  ║     Slim your node_modules by up to 50%      ║
  ║          Created by Jatin Jalandhra          ║
  ╚═══════════════════════════════════════════════╝

  Phase 1: Discovery ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ◉ Scanning node_modules... Found 5,057 files across 71 packages.
  ◉ Total size indexed: 68.7MB

  Phase 2: Simulation ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ◉ Analyzing dependency tree... 2,400 files (35MB) identified.

    ╭────────────────┬───────┬─────────┬────────────╮
    │ Category       ┆ Files ┆ Size    ┆ Risk       │
    ╞════════════════╪═══════╪═════════╪════════════╡
    │ Documentation  ┆ 1,200 ┆ 15MB    ┆ ▪ Low      │
    │ Test-Asset     ┆ 800   ┆ 12MB    ┆ ▪ Low      │
    │ Source-Map     ┆ 400   ┆ 8MB     ┆ ▪▪ Medium  │
    ╰────────────────┴───────┴─────────┴────────────╯

  Phase 3: Confirmation ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  [SAFE] No critical runtime files targeted.

  💾 Total Savings: 35MB (51% of node_modules)
  ℹ This will NOT affect npm start or npm build.

  → Run with --force to execute deletion.
  ✨ Made with ❤️  by Jatin Jalandhra
```

### 2. Execute Deletion

Once you've reviewed the dry run and are satisfied:

```bash
npx jatin-lean --force
```

**What happens:**
- Files are deleted permanently
- Empty directories are cleaned up
- Progress bar shows real-time status
- Summary shows what was deleted

**Output:**
```
  Phase 4: Execution ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ⠋ Cleaning... [██████████████████████████████] 100% | Deleted 35MB

  ✓ Deleted 35MB (2,400 files) in 1.2s

  🎉 Your node_modules is now leaner and faster!
  ✨ Made with ❤️  by Jatin Jalandhra
```

### 3. Verbose Mode

See exactly which files will be deleted:

```bash
npx jatin-lean --verbose
```

**Output includes:**
```
  Files targeted for deletion: ━━━━━━━━━━━━━━━━━━━━━━━━━━━

  ▸ [Documentation]:
    · /path/to/node_modules/express/README.md (15KB)
    · /path/to/node_modules/lodash/CHANGELOG.md (8KB)
    ...and 1,198 more

  ▸ [Test-Asset]:
    · /path/to/node_modules/react/test/utils.js (5KB)
    · /path/to/node_modules/axios/__tests__/http.js (12KB)
    ...and 798 more
```

---

## 🔧 Advanced Usage

### Scan Specific Project

```bash
npx jatin-lean /path/to/project
```

### Global Mode - Scan Multiple Projects

Find all node_modules in a directory and show potential savings:

```bash
npx jatin-lean ~/projects --global
```

**Output:**
```
  System Efficiency Report ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    ╭────────────────┬───────┬───────────────────┬──────────────────╮
    │ Project        ┆ Size  ┆ Potential Savings ┆ Last Accessed    │
    ╞════════════════╪═══════╪═══════════════════╪══════════════════╡
    │ my-app         ┆ 850MB ┆ 400MB             ┆ 2 days ago       │
    │ old-project    ┆ 1.2GB ┆ 600MB             ┆ 142 days ago     │
    │ TOTAL          ┆ 2.0GB ┆ 1.0GB             ┆ —                │
    ╰────────────────┴───────┴───────────────────┴──────────────────╯

  💾 Total potential savings: 1.0GB
  → Run jatin-lean <path> --force on individual projects to prune.
```

### Control Scan Depth

```bash
npx jatin-lean ~/projects --global --max-depth 3
```

---

## 📋 All CLI Options

| Option | Short | Description | Example |
|--------|-------|-------------|---------|
| `--force` | `-f` | Execute deletion (default is dry-run) | `npx jatin-lean --force` |
| `--dry-run` | `-d` | Explicitly run in dry-run mode | `npx jatin-lean --dry-run` |
| `--verbose` | `-v` | Show individual files targeted | `npx jatin-lean --verbose` |
| `--global` | `-g` | Scan all projects in directory | `npx jatin-lean ~/projects --global` |
| `--max-depth N` | | Max directory depth for global scan | `npx jatin-lean --global --max-depth 5` |
| `--help` | `-h` | Show help information | `npx jatin-lean --help` |
| `--version` | `-V` | Show version number | `npx jatin-lean --version` |

---

## 🎯 Common Use Cases

### 1. Clean Up After npm install

```bash
cd my-project
npm install
npx jatin-lean --force
```

### 2. Reduce Docker Image Size

**Dockerfile:**
```dockerfile
FROM node:18-alpine

WORKDIR /app
COPY package*.json ./
RUN npm ci --production

# Reduce image size by ~50%
RUN npx jatin-lean --force

COPY . .
CMD ["npm", "start"]
```

**Result:** Smaller Docker images, faster deployments!

### 3. Automated Cleanup in package.json

**Option A: After every install**
```json
{
  "scripts": {
    "postinstall": "jatin-lean --force"
  }
}
```

**Option B: Manual cleanup command**
```json
{
  "scripts": {
    "clean:modules": "jatin-lean --force",
    "analyze:modules": "jatin-lean --verbose"
  }
}
```

Then run:
```bash
npm run clean:modules
```

### 4. CI/CD Pipeline Optimization

**GitHub Actions:**
```yaml
name: Build and Deploy

on: [push]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install dependencies
        run: npm ci
      
      - name: Optimize node_modules
        run: npx jatin-lean --force
      
      - name: Build
        run: npm run build
      
      - name: Deploy
        run: npm run deploy
```

### 5. Find Old Projects to Clean

```bash
# Scan all your projects
npx jatin-lean ~/projects --global

# Clean old projects that haven't been used
npx jatin-lean ~/projects/old-project --force
```

### 6. Before Committing node_modules (Not Recommended, but...)

If you must commit node_modules:
```bash
npm install --production
npx jatin-lean --force
git add node_modules
git commit -m "Add optimized node_modules"
```

---

## 🛡️ What Gets Deleted?

### Low Risk (Always Safe)

✅ **Documentation Files**
- README.md, CHANGELOG.md, CONTRIBUTING.md
- LICENSE files (kept for legal compliance)
- docs/ directories

✅ **Test Files**
- test/, __tests__/, spec/ directories
- *.test.js, *.spec.js files
- Test fixtures and mocks

✅ **CI/CD Configuration**
- .travis.yml, .circleci/, .github/workflows/
- appveyor.yml, Jenkinsfile

✅ **Example Code**
- example/, examples/, demo/, demos/
- Sample applications

### Medium Risk (Usually Safe)

⚠️ **Source Maps**
- *.js.map, *.css.map
- Useful for debugging, but not needed in production

⚠️ **Build Artifacts**
- *.c, *.cpp, *.o files
- Makefile, binding.gyp
- Native build files (after compilation)

### Higher Risk (Carefully Evaluated)

⚠️⚠️ **TypeScript Sources**
- *.ts, *.tsx files
- **Declaration files (.d.ts) are KEPT**
- Only source files are removed

---

## 🔒 What's Protected?

### Never Deleted

🛡️ **Entry Points**
- Files listed in package.json: `main`, `module`, `browser`
- Binary executables: `bin` field
- Export maps: `exports` field
- Type declarations: `types`, `typings`

🛡️ **Critical Directories**
- `.bin/` directories (npm executables)
- Dotfiles (except .github, .circleci, .travis)

🛡️ **Runtime Files**
- JavaScript files in package root
- Compiled output (dist/, lib/, build/ when they contain entry points)

---

## ⚠️ Safety Guidelines

### Before Using

1. ✅ **Always run dry-run first** (default behavior)
2. ✅ **Review the file list** with `--verbose`
3. ✅ **Test your application** after pruning
4. ✅ **Keep backups** of critical projects

### After Using

1. ✅ **Test your application**
   ```bash
   npm start
   npm test
   npm run build
   ```

2. ✅ **Check if everything works**
   - Run your app
   - Test all features
   - Verify builds complete

3. ✅ **If something breaks**
   ```bash
   # Reinstall dependencies
   rm -rf node_modules
   npm install
   ```

### When NOT to Use

❌ **Don't use if:**
- You need source maps for production debugging
- Your build process requires TypeScript sources
- You're debugging and need test files
- You're unsure about your dependencies

---

## 🐛 Troubleshooting

### "No node_modules found"

**Problem:** Tool can't find node_modules directory

**Solution:**
```bash
# Make sure you're in the right directory
cd /path/to/your/project

# Or specify the path
npx jatin-lean /path/to/project
```

### "Binary not found" or "Permission denied"

**Problem:** Binary is missing or not executable

**Solution:**
```bash
# Reinstall the package
npm uninstall -g jatin-lean
npm install -g jatin-lean

# Or fix permissions
chmod +x $(which jatin-lean)
```

### "Platform not supported"

**Problem:** Currently only Linux x64 is supported

**Solution:**
```bash
# Build from source
git clone https://github.com/your-username/jatin-lean.git
cd jatin-lean
cargo build --release

# Use the binary
./target/release/jatin-lean
```

### Application Breaks After Pruning

**Problem:** Something was deleted that was needed

**Solution:**
```bash
# Reinstall dependencies
rm -rf node_modules
npm install

# Report the issue
# Open an issue on GitHub with details
```

---

## 📊 Performance

### Expected Speed

- **Small projects** (< 100 packages): < 1 second
- **Medium projects** (100-500 packages): 1-3 seconds
- **Large projects** (500+ packages): 3-10 seconds

### Expected Savings

- **Typical projects**: 30-50% reduction
- **Test-heavy projects**: 40-60% reduction
- **Documentation-heavy**: 20-40% reduction

### Benchmarks

| Project Type | Before | After | Savings | Time |
|--------------|--------|-------|---------|------|
| React App | 250MB | 125MB | 50% | 2.1s |
| Express API | 180MB | 95MB | 47% | 1.5s |
| Next.js | 420MB | 230MB | 45% | 3.2s |
| Monorepo | 1.2GB | 650MB | 46% | 8.5s |

---

## 🌍 Platform Support

### Current Version (v0.1.0)

✅ **Linux x64** - Fully supported

### Coming Soon

⏳ **macOS** (Intel & Apple Silicon)
⏳ **Windows** (x64)
⏳ **Linux ARM64**

### Building from Source (All Platforms)

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/your-username/jatin-lean.git
cd jatin-lean
cargo build --release

# Binary at: target/release/jatin-lean
```

---

## 💡 Tips & Best Practices

### 1. Use in Development, Not Production

```bash
# Good: Clean before building for production
npm ci --production
npx jatin-lean --force
npm run build

# Bad: Don't run in production runtime
# (node_modules should already be optimized)
```

### 2. Combine with npm ci

```bash
# Use npm ci for clean installs
npm ci --production
npx jatin-lean --force
```

### 3. Check Savings First

```bash
# See potential savings before committing
npx jatin-lean --verbose
```

### 4. Automate in Docker

```dockerfile
RUN npm ci --production && npx jatin-lean --force
```

### 5. Use Global Mode for Cleanup

```bash
# Find old projects wasting space
npx jatin-lean ~/projects --global
```

---

## 📞 Getting Help

### Documentation

- **This guide**: Complete usage instructions
- **README.md**: Quick start and overview
- **GitHub Issues**: Report bugs or request features

### Common Questions

**Q: Is it safe?**
A: Yes! Entry points are protected, and dry-run is default.

**Q: Will it break my app?**
A: Very unlikely. Only non-runtime files are targeted.

**Q: Can I undo?**
A: No, but you can reinstall: `rm -rf node_modules && npm install`

**Q: Does it work with Yarn/pnpm?**
A: Yes! It works with any package manager.

**Q: Can I customize what gets deleted?**
A: Not yet, but it's planned for future releases.

---

## 🎉 Success Stories

### Example 1: React Application

```bash
$ npx jatin-lean
💾 Total Savings: 125MB (50% of node_modules)

$ npx jatin-lean --force
✓ Deleted 125MB (3,200 files) in 2.1s
```

**Result:** Faster Docker builds, smaller images!

### Example 2: Monorepo

```bash
$ npx jatin-lean ~/monorepo --global
💾 Total potential savings: 2.5GB across 12 projects

$ # Clean each project
$ npx jatin-lean ~/monorepo/app1 --force
$ npx jatin-lean ~/monorepo/app2 --force
```

**Result:** 2.5GB of disk space recovered!

---

## 📄 License

MIT License - Free to use, modify, and distribute.

---

## 🤝 Contributing

Found a bug? Have a suggestion? 

Open an issue on GitHub: https://github.com/your-username/jatin-lean/issues

---

**Made with ❤️ by Jatin Jalandhra**

Thank you for using jatin-lean! 🚀
