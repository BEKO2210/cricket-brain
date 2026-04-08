import { describe, expect, it } from 'vitest';
import { sortProvenanceEntries, summarizeProvenanceSources } from '@/components/sections/provenance-utils';

describe('provenance utility', () => {
  const entries = [
    { metricId: 'zeta', trace: { source: 'tests' as const, rule: 'r', paths: ['tests/a.rs'] } },
    { metricId: 'alpha', trace: { source: 'benchmarks' as const, rule: 'r', paths: ['benchmarks/a.rs'] } },
    { metricId: 'beta', trace: { source: 'tests' as const, rule: 'r', paths: ['tests/b.rs'] } }
  ];

  it('sorts by source and metric id', () => {
    const sorted = sortProvenanceEntries(entries);
    expect(sorted.map((item) => item.metricId)).toEqual(['alpha', 'beta', 'zeta']);
  });

  it('summarizes source counts for compact trust scan', () => {
    const summary = summarizeProvenanceSources(entries);
    expect(summary).toEqual([
      { source: 'tests', count: 2 },
      { source: 'benchmarks', count: 1 }
    ]);
  });
});
