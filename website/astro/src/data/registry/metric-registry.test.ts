import { describe, expect, it } from 'vitest';
import { runMetricRegistry } from '@/data/registry/metric-registry';
import type { MetricProviderContext } from '@/types/metrics';

describe('runMetricRegistry', () => {
  it('produces all default cards with provider ids and traces', () => {
    const context: MetricProviderContext = {
      validated: {
        benchmarkFileCount: 2,
        testFileCount: 3,
        benchmarkCaseCount: 4,
        testCaseCount: 8,
        benchmarkAssertions: 5,
        testAssertions: 16
      },
      benchmarkNames: ['a', 'b'],
      testNames: ['t1', 't2'],
      normalizedPaths: {
        benchmarkPaths: ['benchmarks/a.rs'],
        testPaths: ['tests/t.rs']
      }
    };

    const result = runMetricRegistry(context);

    expect(result.cards.length).toBe(7);
    expect(result.cards.every((card) => card.providerId.length > 0)).toBe(true);
    expect(result.traces.qualityPhiRatio.rule).toContain('φ');
    expect(result.issues).toEqual([]);
  });
});
