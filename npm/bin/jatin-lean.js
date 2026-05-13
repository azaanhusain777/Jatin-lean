#!/usr/bin/env node

/**
 * Wrapper script to execute the jatin-lean binary.
 * This is the entry point when running `npx jatin-lean` or `jatin-lean`.
 */

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

const BINARY_NAME = 'jatin-lean' + (process.platform === 'win32' ? '.exe' : '');
const BINARY_PATH = path.join(__dirname, '..', 'bin', BINARY_NAME);

function runBinary() {
  // Check if binary exists
  if (!fs.existsSync(BINARY_PATH)) {
    console.error('Error: jatin-lean binary not found.');
    console.error('Please run: npm install jatin-lean');
    console.error(`Expected location: ${BINARY_PATH}`);
    process.exit(1);
  }

  // Check if binary is executable
  try {
    fs.accessSync(BINARY_PATH, fs.constants.X_OK);
  } catch (error) {
    console.error('Error: Binary is not executable.');
    console.error('Attempting to fix permissions...');
    try {
      fs.chmodSync(BINARY_PATH, 0o755);
      console.log('✓ Permissions fixed.');
    } catch (chmodError) {
      console.error('Failed to fix permissions:', chmodError.message);
      process.exit(1);
    }
  }

  // Pass all arguments to the binary
  const args = process.argv.slice(2);
  
  const child = spawn(BINARY_PATH, args, {
    stdio: 'inherit',
    env: process.env,
  });

  child.on('error', (error) => {
    console.error('Failed to start jatin-lean:', error.message);
    process.exit(1);
  });

  child.on('exit', (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
    } else {
      process.exit(code || 0);
    }
  });
}

runBinary();
