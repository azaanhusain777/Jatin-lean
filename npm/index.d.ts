/**
 * TypeScript definitions for jatin-lean
 * 
 * @module jatin-lean
 */

/** Output from an async command execution. */
export interface CommandResult {
  stdout: string;
  stderr: string;
  exitCode: number;
}

/** Scan results object. */
export interface ScanResults {
  scan: {
    root: string;
    total_files: number;
    total_size: number;
    total_size_human: string;
    total_packages: number;
  };
  results: {
    candidate_count: number;
    candidate_size: number;
    candidate_size_human: string;
    savings_percentage: string;
    risk_level: string;
    categories: Record<string, {
      count: number;
      size: number;
      size_human: string;
    }>;
  };
  timestamp: string;
}

/** Savings calculation result. */
export interface SavingsInfo {
  candidateCount: number;
  candidateSize: number;
  candidateSizeHuman: string;
  savingsPercentage: string;
  riskLevel: string;
  totalSize: number;
  totalSizeHuman: string;
}

/** Options for the scan function. */
export interface ScanOptions {
  /** Path to custom config file. */
  config?: string;
}

/** Options for the prune function. */
export interface PruneOptions {
  /** Create a snapshot before pruning. */
  snapshot?: boolean;
  /** Path to custom config file. */
  config?: string;
}

/** Options for the spawn wrapper. */
export interface SpawnOptions {
  env?: Record<string, string>;
  cwd?: string;
}

// ─── Core Functions ──────────────────────────────────────────────────────────

/**
 * Execute jatin-lean with the given arguments (async/promise).
 */
export function run(args?: string[], options?: SpawnOptions): Promise<CommandResult>;

/**
 * Execute jatin-lean with the given arguments (synchronous).
 */
export function runSync(args?: string[], options?: object): string;

/**
 * Check if the jatin-lean binary is available.
 */
export function isInstalled(): boolean;

/**
 * Get the version of the installed binary.
 */
export function getVersion(): string | null;

/** Absolute path to the binary. */
export const binaryPath: string;

// ─── High-level API ──────────────────────────────────────────────────────────

/**
 * Scan a project's node_modules and return results as JSON.
 */
export function scan(projectPath?: string, options?: ScanOptions): Promise<ScanResults | null>;

/**
 * Prune a project's node_modules (actually delete files).
 */
export function prune(projectPath?: string, options?: PruneOptions): Promise<CommandResult>;

/**
 * Calculate potential savings without modifying anything.
 */
export function calculateSavings(projectPath?: string): Promise<SavingsInfo | null>;

/**
 * Scan all node_modules in a directory tree.
 */
export function globalScan(rootPath?: string, maxDepth?: number): Promise<CommandResult>;

// ─── Analysis ────────────────────────────────────────────────────────────────

/**
 * Find duplicate files in node_modules.
 */
export function findDuplicates(projectPath?: string): Promise<CommandResult>;

/**
 * Analyze the dependency graph.
 */
export function analyzeDeps(projectPath?: string): Promise<CommandResult>;

// ─── Snapshots ───────────────────────────────────────────────────────────────

/**
 * List available snapshots.
 */
export function listSnapshots(): Promise<CommandResult>;

/**
 * Restore a snapshot.
 */
export function restoreSnapshot(snapshotId: string): Promise<CommandResult>;

// ─── Analytics ───────────────────────────────────────────────────────────────

/**
 * Get analytics dashboard data.
 */
export function getAnalytics(): Promise<CommandResult>;

// ─── Configuration ───────────────────────────────────────────────────────────

/**
 * Generate an example configuration file.
 */
export function initConfig(outputPath?: string): Promise<CommandResult>;

/**
 * Create a postinstall hook script command.
 */
export function getPostInstallScript(): string;
