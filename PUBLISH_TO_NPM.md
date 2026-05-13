# 📦 Publishing jatin-lean to npm - Step by Step Guide

## ✅ Pre-Publishing Checklist

Everything is ready! ✅
- [x] Binary built and optimized (2.9MB)
- [x] npm package structure complete
- [x] README.md created
- [x] package.json configured
- [x] Author name: Jatin Jalandhra
- [x] License: MIT
- [x] All tests passing

---

## 🚀 Publishing Steps

### Step 1: Create npm Account (if you don't have one)

1. Go to https://www.npmjs.com/signup
2. Fill in:
   - Username (e.g., `jatinjalandhra`)
   - Email
   - Password
3. Verify your email

### Step 2: Login to npm

Open your terminal and run:

```bash
npm login
```

You'll be prompted for:
- **Username**: Your npm username
- **Password**: Your npm password  
- **Email**: Your email (this will be public)
- **OTP**: One-time password (if you have 2FA enabled)

### Step 3: Check if Package Name is Available

```bash
npm view jatin-lean
```

**Expected output:**
- If you see `npm ERR! 404 Not Found` → **Name is available!** ✅
- If you see package info → **Name is taken** ❌

**If name is taken**, you have two options:

**Option A: Use a different name**
```bash
# Edit npm/package.json and change "name" to something else
# Examples: jatin-lean-cli, node-lean, jatin-prune
```

**Option B: Use a scoped package**
```bash
# Edit npm/package.json
# Change: "name": "jatin-lean"
# To: "name": "@your-username/jatin-lean"

# Then publish with:
npm publish --access public
```

### Step 4: Final Verification

```bash
cd npm

# Check what will be published
npm pack --dry-run

# You should see:
# ✅ README.md
# ✅ LICENSE
# ✅ package.json
# ✅ install.js
# ✅ index.js
# ✅ bin/jatin-lean
# ✅ bin/jatin-lean.js
```

### Step 5: Publish! 🎉

```bash
cd npm
npm publish
```

**Expected output:**
```
npm notice
npm notice 📦  jatin-lean@0.1.0
npm notice === Tarball Contents ===
npm notice 1.1kB LICENSE
npm notice 5.8kB README.md
npm notice 3.0MB bin/jatin-lean
npm notice 1.6kB bin/jatin-lean.js
npm notice 738B  index.js
npm notice 3.7kB install.js
npm notice 1.1kB package.json
npm notice === Tarball Details ===
npm notice name:          jatin-lean
npm notice version:       0.1.0
npm notice package size:  1.3 MB
npm notice unpacked size: 3.0 MB
npm notice total files:   7
npm notice
+ jatin-lean@0.1.0
```

### Step 6: Verify Publication

```bash
# Check on npm
npm view jatin-lean

# Test installation
npx jatin-lean@latest --help

# Or install globally
npm install -g jatin-lean
jatin-lean --help
```

---

## 🎉 Success! What Happens Next?

Once published, users can:

```bash
# Run directly without installing
npx jatin-lean

# Install globally
npm install -g jatin-lean

# Install as dev dependency
npm install --save-dev jatin-lean
```

---

## 📊 After Publishing

### View Your Package

- **npm page**: https://www.npmjs.com/package/jatin-lean
- **Download stats**: https://npm-stat.com/charts.html?package=jatin-lean

### Update Your README

Add the npm badge to your main README.md:

```markdown
[![npm version](https://img.shields.io/npm/v/jatin-lean.svg)](https://www.npmjs.com/package/jatin-lean)
[![npm downloads](https://img.shields.io/npm/dm/jatin-lean.svg)](https://www.npmjs.com/package/jatin-lean)
```

### Share Your Tool

- Tweet about it
- Post on Reddit (r/node, r/javascript, r/rust)
- Share on LinkedIn
- Post on Hacker News
- Add to awesome lists

---

## 🔄 Publishing Updates

When you make changes:

```bash
# Update version
cd npm
npm version patch  # 0.1.0 -> 0.1.1
# or
npm version minor  # 0.1.0 -> 0.2.0
# or
npm version major  # 0.1.0 -> 1.0.0

# Rebuild binary
cd ..
cargo build --release
cp target/release/jatin-lean npm/bin/

# Publish
cd npm
npm publish
```

---

## ⚠️ Troubleshooting

### "You do not have permission to publish"

```bash
# Make sure you're logged in
npm whoami

# If not logged in
npm login
```

### "Package name too similar to existing package"

npm might reject names that are too similar to existing packages. Try:
- Adding a prefix/suffix: `jatin-lean-cli`
- Using a scoped package: `@your-username/jatin-lean`

### "You must verify your email"

Check your email and click the verification link from npm.

### "Package name already exists"

The name is taken. Use a different name or scoped package.

---

## 🎯 Quick Command Reference

```bash
# Login
npm login

# Check if name is available
npm view jatin-lean

# Dry run (see what will be published)
cd npm && npm pack --dry-run

# Publish
cd npm && npm publish

# Publish scoped package
cd npm && npm publish --access public

# Test after publishing
npx jatin-lean@latest --help

# View package info
npm view jatin-lean

# Check who you're logged in as
npm whoami
```

---

## 📝 Important Notes

1. **Email will be public**: Your email in npm account will be visible on the package page
2. **Cannot unpublish after 72 hours**: You can only unpublish within 72 hours of publishing
3. **Version numbers are permanent**: Once published, you cannot reuse a version number
4. **Package name is permanent**: You cannot rename a package after publishing

---

## ✨ You're Ready!

Everything is set up and ready to publish. Just run:

```bash
npm login
cd npm
npm publish
```

Good luck with your launch! 🚀

---

**Made with ❤️ by Jatin Jalandhra**
