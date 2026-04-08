import type { SourceTrace } from '@/types/metrics';

export type ProvenanceEntry = {
  metricId: string;
  trace: SourceTrace;
};

export const sortProvenanceEntries = (entries: ProvenanceEntry[]): ProvenanceEntry[] =>
  [...entries].sort((a, b) => {
    if (a.trace.source === b.trace.source) {
      return a.metricId.localeCompare(b.metricId);
    }
    return a.trace.source.localeCompare(b.trace.source);
  });

export const summarizeProvenanceSources = (entries: ProvenanceEntry[]): Array<{ source: string; count: number }> => {
  const counts = new Map<string, number>();
  for (const entry of entries) {
    counts.set(entry.trace.source, (counts.get(entry.trace.source) ?? 0) + 1);
  }

  return [...counts.entries()]
    .map(([source, count]) => ({ source, count }))
    .sort((a, b) => b.count - a.count || a.source.localeCompare(b.source));
};
