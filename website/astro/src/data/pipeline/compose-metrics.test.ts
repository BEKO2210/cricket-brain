import { describe, expect, it } from 'vitest';
import { composeLandingMetrics } from '@/data/pipeline/compose-metrics';

describe('composeLandingMetrics', () => {
  it('aggregates inherited and registry issues and returns traces', () => {
    const result = composeLandingMetrics(
      {
        benchmarkFileCount: 1,
        testFileCount: 1,
        benchmarkCaseCount: 2,
        testCaseCount: 4,
        benchmarkAssertions: 3,
        testAssertions: 8
      },
      ['b1'],
      ['t1'],
      { benchmarkPaths: ['benchmarks/b1.rs'], testPaths: ['tests/t1.rs'] },
      [{ level: 'warning', message: 'inherited issue' }]
    );

    expect(result.cards.length).toBeGreaterThan(0);
    expect(result.issues.map((issue) => issue.message)).toContain('inherited issue');
    expect(result.traces.benchmarkFiles.paths[0]).toBe('benchmarks/b1.rs');
  });
});
