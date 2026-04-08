import type { ExtractedSignals, RawRustSource } from '@/types/metrics';

const TEST_FUNCTION_PATTERN = /#\[test\][\s\S]*?fn\s+([a-zA-Z0-9_]+)\s*\(/g;
const BENCH_FUNCTION_PATTERN = /bench_function\s*\(|criterion_group!\s*\(|fn\s+([a-zA-Z0-9_]+)_benchmark\s*\(/g;
const ASSERT_PATTERN = /(assert!|assert_eq!|assert_ne!|expect\()/g;

const uniqueSorted = (input: string[]): string[] => Array.from(new Set(input)).sort((a, b) => a.localeCompare(b));

const normalizeToken = (token: string): string => token.replace(/\s*\(.*/, '').trim();

const extractMatches = (files: RawRustSource[], pattern: RegExp): string[] => {
  const found: string[] = [];
  for (const file of files) {
    for (const match of file.content.matchAll(pattern)) {
      const capturedName = match[1];
      const rawMatch = match[0];
      const value = capturedName ?? normalizeToken(rawMatch);
      if (value) {
        found.push(value);
      }
    }
  }
  return uniqueSorted(found);
};

const countAssertions = (files: RawRustSource[]): number =>
  files.reduce((total: number, file: RawRustSource) => total + (file.content.match(ASSERT_PATTERN)?.length ?? 0), 0);

export const extractSignals = (benchmarkFiles: RawRustSource[], testFiles: RawRustSource[]): ExtractedSignals => ({
  benchmarkFiles,
  testFiles,
  benchmarkFunctionNames: extractMatches(benchmarkFiles, BENCH_FUNCTION_PATTERN),
  testFunctionNames: extractMatches(testFiles, TEST_FUNCTION_PATTERN),
  benchmarkAssertions: countAssertions(benchmarkFiles),
  testAssertions: countAssertions(testFiles)
});
