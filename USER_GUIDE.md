# 📘 jatin-lean User Guide

Welcome to the official user guide for **jatin-lean**! This guide will help you understand how to use the tool to significantly reduce the size of your `node_modules` folders, freeing up valuable disk space and making your projects more lightweight.

---

## 🌟 What is jatin-lean?

`jatin-lean` is a high-performance CLI utility that intelligently scans your `node_modules` directory and removes non-essential, non-runtime files. These include:

*   **Documentation:** `README.md`, `CHANGELOG.md`, `CONTRIBUTING.md`
*   **Test Assets:** `test/`, `__tests__/`, `*.test.js`
*   **Source Maps:** `*.js.map`, `*.css.map`
*   **TypeScript Sources:** `*.ts` (while keeping essential `.d.ts` declaration files)
*   **Build Artifacts & Examples:** Uncompiled C/C++ files, Makefile, `example/` directories

It safely cleans these files without breaking your application's ability to run (`npm start`) or build (`npm run build`).

---

## 🚀 Quick Start (No Installation Required)

The easiest way to use `jatin-lean` is via `npx`. This downloads and runs the tool temporarily without permanently installing it.

1. Open your terminal.
2. Navigate to your Node.js project directory (the folder containing `node_modules`).
   ```bash
   cd /path/to/your/project
   ```
3. Run `jatin-lean` using `npx`:
   ```bash
   npx jatin-lean
   ```

**Don't worry!** By default, `jatin-lean` runs in **Dry-Run Mode**. It will only analyze your `node_modules` and show you exactly what *would* be deleted and how much space you *would* save. It will not actually delete anything until you tell it to.

---

## 💾 Installation (Optional)

If you find yourself using `jatin-lean` frequently across multiple projects, you might want to install it globally on your machine.

```bash
npm install -g jatin-lean
```

Once installed globally, you can simply run it by typing:
```bash
jatin-lean
```

---

## 🛠️ How to Use

### 1. The Dry Run (Simulation)
Always start with a dry run to see what the tool will do.
```bash
# Using npx
npx jatin-lean

# If installed globally
jatin-lean
```
This will output a summary table showing categories of files, the number of files, and the potential disk space savings.

### 2. Execute Deletion (The Real Deal)
Once you are satisfied with the simulation, add the `--force` (or `-f`) flag to actually perform the deletion.
```bash
# Using npx
npx jatin-lean --force

# If installed globally
jatin-lean --force
```
*Note: This action cannot be undone. You will need to run `npm install` again to restore the deleted files.*

### 3. See Exactly What Will Be Deleted (Verbose Mode)
If you want to see a detailed list of every single file that `jatin-lean` targets, use the `--verbose` (or `-v`) flag.
```bash
npx jatin-lean --verbose
```
You can combine flags too:
```bash
npx jatin-lean --verbose --force
```

---

## 🌍 Global Mode: Clean Multiple Projects at Once

If you have a directory containing multiple Node.js projects (e.g., a `~/projects` folder), `jatin-lean` can scan all of them at once to calculate total potential savings.

```bash
# Using npx
npx jatin-lean /path/to/your/projects/folder --global

# Example
npx jatin-lean ~/Development --global
```

This will output a "System Efficiency Report" listing all discovered projects and the potential savings for each.

*Note: You can control how deep it searches using the `--max-depth` flag (default is 4).*
```bash
npx jatin-lean ~/projects --global --max-depth 2
```

---

## 🛡️ Is it Safe?

Yes! `jatin-lean` has built-in safety mechanisms:
1.  **Reads `package.json`**: It analyzes the entry points (`main`, `module`, `exports`, etc.) of every package.
2.  **Traces Dependencies**: It traces what files are actually imported or required at runtime.
3.  **Auto-whitelists**: Any file required for runtime is locked and protected from deletion.
4.  **Ignores Critical Folders**: It never touches `.bin/` directories or critical dotfiles.

---

## 📚 Command Reference

| Command / Flag | Short | What it does |
| :--- | :--- | :--- |
| `jatin-lean` | | Runs a dry-run in the current directory. |
| `--force` | `-f` | Executes the deletion. **(Destructive)** |
| `--dry-run` | `-d` | Explicitly enforces a dry-run (default behavior). |
| `--verbose` | `-v` | Prints a detailed list of targeted files. |
| `--global` | `-g` | Scans all sub-directories for `node_modules` folders. |
| `--max-depth <N>`| | Sets how deep to search in `--global` mode. |
| `--help` | `-h` | Prints the help menu. |
| `--version` | `-V` | Prints the current version of the tool. |

---

Enjoy your newly lightweight `node_modules`! ⚡
