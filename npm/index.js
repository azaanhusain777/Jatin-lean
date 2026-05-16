/**
 * jatin-lean — Programmatic API for Node.js
 *
 * Provides a comprehensive API for integrating jatin-lean into
 * build tools, CI/CD pipelines, and other Node.js applications.
 */

const { execSync, spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

const BINARY_NAME = 'jatin-lean' + (process.platform === 'win32' ? '.exe' : '');
const BINARY_PATH = path.join(__dirname, 'bin', BINARY_NAME);

/**
 * Check if the jatin-lean binary is available.
 * @returns {boolean}
 */
function isInstalled() {
  return fs.existsSync(BINARY_PATH);
}

/**
 * Get the version of the installed binary.
 * @returns {string|null}
 */
function getVersion() {
  if (!isInstalled()) return null;
  try {
    const output = execSync(`"${BINARY_PATH}" --version`, { encoding: 'utf8' });
    return output.trim().replace('jatin-lean ', '');
  } catch {
    return null;
  }
}

/**
 * Execute jatin-lean with the given arguments (synchronous).
 * @param {string[]} args - Command line arguments
 * @param {object} options - execSync options
 * @returns {Buffer|string} - Command output
 */
function runSync(args = [], options = {}) {
  if (!isInstalled()) {
    throw new Error('jatin-lean binary not found. Run: npm install jatin-lean');
  }
  const command = `"${BINARY_PATH}" ${args.join(' ')}`;
  return execSync(command, {
    stdio: 'pipe',
    encoding: 'utf8',
    ...options,
  });
}

/**
 * Execute jatin-lean with the given arguments (async/promise).
 * @param {string[]} args - Command line arguments
 * @param {object} options - spawn options
 * @returns {Promise<{stdout: string, stderr: string, exitCode: number}>}
 */
function run(args = [], options = {}) {
  return new Promise((resolve, reject) => {
    if (!isInstalled()) {
      return reject(new Error('jatin-lean binary not found. Run: npm install jatin-lean'));
    }

    let stdout = '';
    let stderr = '';

    const child = spawn(BINARY_PATH, args, {
      env: process.env,
      ...options,
    });

    child.stdout.on('data', (data) => { stdout += data.toString(); });
    child.stderr.on('data', (data) => { stderr += data.toString(); });

    child.on('error', reject);
    child.on('close', (code) => {
      resolve({ stdout, stderr, exitCode: code || 0 });
    });
  });
}

/**
 * Scan a project's node_modules and return results as JSON.
 * @param {string} projectPath - Path to the project directory
 * @param {object} options - Additional options
 * @param {string} [options.config] - Path to custom config file
 * @returns {Promise<object>} - Scan results as parsed JSON
 */
async function scan(projectPath = '.', options = {}) {
  const args = [projectPath, '--export', '/dev/stdout'];
  if (options.config) {
    args.push('--config', options.config);
  }

  // For scan, we use the export to JSON approach
  const exportFile = path.join(require('os').tmpdir(), `jatin-lean-scan-${Date.now()}.json`);
  const scanArgs = [projectPath, '--export', exportFile];
  if (options.config) {
    scanArgs.push('--config', options.config);
  }

  try {
    await run(scanArgs);
    if (fs.existsSync(exportFile)) {
      const content = fs.readFileSync(exportFile, 'utf8');
      fs.unlinkSync(exportFile);
      return JSON.parse(content);
    }
    return null;
  } catch (err) {
    // Clean up temp file
    try { fs.unlinkSync(exportFile); } catch {}
    throw err;
  }
}

/**
 * Prune a project's node_modules (actually delete files).
 * @param {string} projectPath - Path to the project directory
 * @param {object} options - Additional options
 * @param {boolean} [options.snapshot=false] - Create snapshot before pruning
 * @param {string} [options.config] - Path to custom config file
 * @returns {Promise<{stdout: string, stderr: string, exitCode: number}>}
 */
async function prune(projectPath = '.', options = {}) {
  const args = [projectPath, '--force', '--yes'];
  if (options.snapshot) {
    args.push('--snapshot');
  }
  if (options.config) {
    args.push('--config', options.config);
  }
  return run(args);
}

/**
 * Find duplicate files in node_modules.
 * @param {string} projectPath - Path to the project directory
 * @returns {Promise<{stdout: string, stderr: string, exitCode: number}>}
 */
async function findDuplicates(projectPath = '.') {
  return run(['dedup', projectPath]);
}

/**
 * Analyze the dependency graph.
 * @param {string} projectPath - Path to the project directory
 * @returns {Promise<{stdout: string, stderr: string, exitCode: number}>}
 */
async function analyzeDeps(projectPath = '.') {
  return run(['deps', projectPath]);
}

/**
 * Get analytics dashboard data.
 * @returns {Promise<{stdout: string, stderr: string, exitCode: number}>}
 */
async function getAnalytics() {
  return run(['analytics']);
}

/**
 * List available snapshots.
 * @returns {Promise<{stdout: string, stderr: string, exitCode: number}>}
 */
async function listSnapshots() {
  return run(['snapshots', '--list']);
}

/**
 * Restore a snapshot.
 * @param {string} snapshotId - The snapshot ID to restore
 * @returns {Promise<{stdout: string, stderr: string, exitCode: number}>}
 */
async function restoreSnapshot(snapshotId) {
  return run(['snapshots', '--restore', snapshotId]);
}

/**
 * Scan all node_modules in a directory tree.
 * @param {string} rootPath - Root directory to scan
 * @param {number} [maxDepth=4] - Maximum directory depth
 * @returns {Promise<{stdout: string, stderr: string, exitCode: number}>}
 */
async function globalScan(rootPath = '.', maxDepth = 4) {
  return run([rootPath, '--global', '--max-depth', maxDepth.toString()]);
}

/**
 * Generate an example configuration file.
 * @param {string} outputPath - Where to write the config file
 * @returns {Promise<{stdout: string, stderr: string, exitCode: number}>}
 */
async function initConfig(outputPath = 'jatin-lean.toml') {
  return run(['--init-config', outputPath]);
}

/**
 * Create a postinstall hook script for package.json.
 * Add this to your package.json scripts:
 *   "postinstall": "jatin-lean --force --yes"
 *
 * @returns {string} - The postinstall script command
 */
function getPostInstallScript() {
  return 'jatin-lean --force --yes';
}

/**
 * Calculate potential savings without modifying anything.
 * @param {string} projectPath - Path to the project
 * @returns {Promise<object|null>} - Savings info or null
 */
async function calculateSavings(projectPath = '.') {
  try {
    const result = await scan(projectPath);
    if (result && result.results) {
      return {
        candidateCount: result.results.candidate_count,
        candidateSize: result.results.candidate_size,
        candidateSizeHuman: result.results.candidate_size_human,
        savingsPercentage: result.results.savings_percentage,
        riskLevel: result.results.risk_level,
        totalSize: result.scan.total_size,
        totalSizeHuman: result.scan.total_size_human,
      };
    }
    return null;
  } catch {
    return null;
  }
}

module.exports = {
  // Core
  run,
  runSync,
  isInstalled,
  getVersion,
  binaryPath: BINARY_PATH,

  // High-level API
  scan,
  prune,
  calculateSavings,
  globalScan,

  // Analysis
  findDuplicates,
  analyzeDeps,

  // Snapshots
  listSnapshots,
  restoreSnapshot,

  // Analytics
  getAnalytics,

  // Configuration
  initConfig,
  getPostInstallScript,
};
