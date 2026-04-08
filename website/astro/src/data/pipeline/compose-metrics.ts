import { runMetricRegistry } from '@/data/registry/metric-registry';
import type { LandingMetrics, MetricProviderContext, PipelineIssue, ValidatedSignals } from '@/types/metrics';

export const composeLandingMetrics = (
  validated: ValidatedSignals,
  benchmarkNames: string[],
  testNames: string[],
  normalizedPaths: { benchmarkPaths: string[]; testPaths: string[] },
  inheritedIssues: PipelineIssue[]
): LandingMetrics => {
  const registryContext: MetricProviderContext = {
    validated,
    benchmarkNames,
    testNames,
    normalizedPaths
  };

  const registry = runMetricRegistry(registryContext);

  return {
    updatedAt: new Date().toISOString(),
    cards: registry.cards,
    benchmarkNames,
    testNames,
    issues: [...inheritedIssues, ...registry.issues],
    traces: registry.traces
  };
};
