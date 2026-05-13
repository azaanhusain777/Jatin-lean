/**
 * Main entry point for programmatic usage (if needed in the future).
 * For now, this package is primarily a CLI tool.
 */

const { execSync } = require('child_process');
const path = require('path');

const BINARY_PATH = path.join(__dirname, 'bin', 'jatin-lean' + (process.platform === 'win32' ? '.exe' : ''));

/**
 * Execute jatin-lean with the given arguments.
 * @param {string[]} args - Command line arguments
 * @param {object} options - Execution options
 * @returns {Buffer} - Command output
 */
function run(args = [], options = {}) {
  const command = `"${BINARY_PATH}" ${args.join(' ')}`;
  return execSync(command, {
    stdio: 'inherit',
    ...options,
  });
}

module.exports = {
  run,
  binaryPath: BINARY_PATH,
};
