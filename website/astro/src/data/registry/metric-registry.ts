import { metricProviders } from '@/data/registry/providers';
import type { MetricCard, MetricProviderContext, PipelineIssue, SourceTrace } from '@/types/metrics';

type RegistryOutput = {
  cards: MetricCard[];
  traces: Record<string, SourceTrace>;
  issues: PipelineIssue[];
};

export const runMetricRegistry = (context: MetricProviderContext): RegistryOutput => {
  const cards: MetricCard[] = [];
  const traces: Record<string, SourceTrace> = {};
  const issues: PipelineIssue[] = [];

  for (const provider of metricProviders) {
    try {
      const result = provider.produce(context);
      if (result.issues?.length) {
        issues.push(...result.issues.map((issue) => ({ ...issue, context: issue.context ?? `provider:${provider.id}` })));
      }

      if (!result.card) {
        issues.push({ level: 'warning', message: 'Provider produced no metric card.', context: `provider:${provider.id}` });
        continue;
      }

      cards.push(result.card);
      traces[result.card.id] = result.card.trace;
    } catch (error) {
      issues.push({
        level: 'error',
        message: 'Metric provider failed.',
        context: `provider:${provider.id} (${error instanceof Error ? error.message : 'unknown error'})`
      });
    }
  }

  return { cards, traces, issues };
};
