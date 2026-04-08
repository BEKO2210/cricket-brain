import path from 'node:path';
import { fileURLToPath } from 'node:url';

/**
 * Website boundary contract:
 * - Astro app lives in /website/astro
 * - Source project outside this folder is READ-ONLY input
 * - Only listed source directories may be read by the website data pipeline
 */
const websiteRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
export const repositoryRoot = path.resolve(websiteRoot, '..', '..');

export const readOnlySourceDirectories = {
  benchmarks: path.join(repositoryRoot, 'benchmarks'),
  tests: path.join(repositoryRoot, 'tests')
} as const;

export const sourceGlobs = ['*.rs'] as const;
