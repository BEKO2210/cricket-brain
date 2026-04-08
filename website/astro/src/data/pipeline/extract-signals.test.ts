import { describe, expect, it } from 'vitest';
import { extractSignals } from '@/data/pipeline/extract-signals';
import type { RawRustSource } from '@/types/metrics';

describe('extractSignals', () => {
  it('extracts benchmark/test names and assertion counts from fixtures', () => {
    const bench: RawRustSource[] = [
      {
        path: '/repo/benchmarks/sample.rs',
        filename: 'sample.rs',
        content: 'fn gap_detection_benchmark() {}\ncriterion_group!(benches, gap_detection_benchmark);\nassert_eq!(1,1);'
      }
    ];

    const tests: RawRustSource[] = [
      {
        path: '/repo/tests/sample.rs',
        filename: 'sample.rs',
        content: '#[test]\nfn handles_signal() { assert!(true); assert_ne!(1, 2); }'
      }
    ];

    const result = extractSignals(bench, tests);

    expect(result.benchmarkFunctionNames).toEqual(['criterion_group!', 'gap_detection']);
    expect(result.testFunctionNames).toEqual(['handles_signal']);
    expect(result.benchmarkAssertions).toBe(1);
    expect(result.testAssertions).toBe(2);
  });
});
