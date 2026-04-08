import { describe, expect, it } from 'vitest';
import { validateSignals } from '@/data/pipeline/validate-signals';
import type { ExtractedSignals } from '@/types/metrics';

describe('validateSignals', () => {
  it('emits warnings when source files are missing', () => {
    const signals: ExtractedSignals = {
      benchmarkFiles: [],
      testFiles: [],
      benchmarkFunctionNames: [],
      testFunctionNames: [],
      benchmarkAssertions: 0,
      testAssertions: 0
    };

    const result = validateSignals(signals);
    expect(result.issues.map((issue) => issue.message)).toContain('No benchmark files were available for parsing.');
    expect(result.issues.map((issue) => issue.message)).toContain('No test files were available for parsing.');
  });

  it('keeps validated counts consistent', () => {
    const signals: ExtractedSignals = {
      benchmarkFiles: [{ path: 'a', filename: 'a.rs', content: '' }],
      testFiles: [{ path: 'b', filename: 'b.rs', content: '' }],
      benchmarkFunctionNames: ['bench_a'],
      testFunctionNames: ['test_a', 'test_b'],
      benchmarkAssertions: 2,
      testAssertions: 5
    };

    const result = validateSignals(signals);
    expect(result.validated.testCaseCount).toBe(2);
    expect(result.validated.benchmarkAssertions).toBe(2);
  });
});
