#!/usr/bin/env node

/**
 * Post-install script to download the appropriate binary for the platform.
 * This runs automatically after `npm install jatin-lean`.
 */

const fs = require('fs');
const path = require('path');
const https = require('https');
const { execSync } = require('child_process');

const PACKAGE_VERSION = require('./package.json').version;
const BINARY_NAME = 'jatin-lean';

// Platform-specific binary names
const PLATFORM_MAP = {
  'linux-x64': 'jatin-lean-linux-x64',
  'linux-arm64': 'jatin-lean-linux-arm64',
  'darwin-x64': 'jatin-lean-macos-x64',
  'darwin-arm64': 'jatin-lean-macos-arm64',
  'win32-x64': 'jatin-lean-windows-x64.exe',
};

function getPlatformKey() {
  const platform = process.platform;
  const arch = process.arch;
  return `${platform}-${arch}`;
}

function getBinaryName() {
  const platformKey = getPlatformKey();
  const binaryName = PLATFORM_MAP[platformKey];
  
  if (!binaryName) {
    console.error(`Unsupported platform: ${platformKey}`);
    console.error('Supported platforms:', Object.keys(PLATFORM_MAP).join(', '));
    process.exit(1);
  }
  
  return binaryName;
}

function getBinaryPath() {
  return path.join(__dirname, 'bin', BINARY_NAME + (process.platform === 'win32' ? '.exe' : ''));
}

function downloadBinary(url, dest) {
  return new Promise((resolve, reject) => {
    console.log(`Downloading ${url}...`);
    
    const file = fs.createWriteStream(dest);
    
    https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        // Follow redirect
        https.get(response.headers.location, (redirectResponse) => {
          redirectResponse.pipe(file);
          file.on('finish', () => {
            file.close();
            resolve();
          });
        }).on('error', reject);
      } else if (response.statusCode === 200) {
        response.pipe(file);
        file.on('finish', () => {
          file.close();
          resolve();
        });
      } else {
        reject(new Error(`Failed to download: HTTP ${response.statusCode}`));
      }
    }).on('error', reject);
  });
}

async function install() {
  const binaryName = getBinaryName();
  const binaryPath = getBinaryPath();
  const binDir = path.dirname(binaryPath);
  
  // Create bin directory if it doesn't exist
  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }
  
  // Check if binary already exists (for local development)
  if (fs.existsSync(binaryPath)) {
    console.log('Binary already exists, skipping download.');
    fs.chmodSync(binaryPath, 0o755);
    return;
  }
  
  // For now, try to copy from local build if available (development mode)
  const localBinaryPath = path.join(__dirname, '..', 'target', 'release', BINARY_NAME);
  if (fs.existsSync(localBinaryPath)) {
    console.log('Using local development binary...');
    fs.copyFileSync(localBinaryPath, binaryPath);
    fs.chmodSync(binaryPath, 0o755);
    console.log('✓ Installation complete!');
    return;
  }
  
  // Check if binary is already bundled (for the platform we published from)
  const bundledBinaryPath = path.join(__dirname, 'bin', BINARY_NAME);
  if (fs.existsSync(bundledBinaryPath)) {
    console.log('Using bundled binary...');
    if (bundledBinaryPath !== binaryPath) {
      fs.copyFileSync(bundledBinaryPath, binaryPath);
    }
    fs.chmodSync(binaryPath, 0o755);
    console.log('✓ Installation complete!');
    return;
  }
  
  // Download from GitHub releases
  const downloadUrl = `https://github.com/jatinjalandhra/jatin-lean/releases/download/v${PACKAGE_VERSION}/${binaryName}`;
  
  console.log(`Downloading binary for ${getPlatformKey()}...`);
  
  try {
    await downloadBinary(downloadUrl, binaryPath);
    fs.chmodSync(binaryPath, 0o755);
    console.log('✓ Installation complete!');
  } catch (error) {
    console.error('\n❌ Failed to download binary:', error.message);
    console.error(`\nTried to download from: ${downloadUrl}`);
    console.error('\nPlease try one of the following:');
    console.error('1. Check your internet connection');
    console.error('2. Download manually from: https://github.com/jatinjalandhra/jatin-lean/releases');
    console.error(`3. Build from source: https://github.com/jatinjalandhra/jatin-lean#building-from-source`);
    process.exit(1);
  }
}

// Run installation
install().catch((error) => {
  console.error('Installation failed:', error);
  process.exit(1);
});
