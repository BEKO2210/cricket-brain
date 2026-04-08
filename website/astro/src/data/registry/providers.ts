import { PHI } from '@/lib/phi';
import type { MetricProvider } from '@/types/metrics';

const average = (sum: number, count: number): number => (count > 0 ? Number((sum / count).toFixed(2)) : 0);

export const metricProviders: MetricProvider[] = [
  {
    id: 'benchmark.files',
    produce: ({ validated, normalizedPaths }) => ({
      card: {
        id: 'benchmarkFiles',
        providerId: 'benchmark.files',
        labelKey: 'metrics.benchmarkFiles',
        detailKey: 'metricsDetail.benchmarkFiles',
        value: validated.benchmarkFileCount,
        unitKey: 'units.files',
        trace: {
          source: 'benchmarks',
          rule: 'Count readable .rs files in /benchmarks.',
          paths: normalizedPaths.benchmarkPaths
        }
      }
    })
  },
  {
    id: 'benchmark.cases',
    produce: ({ validated, normalizedPaths }) => ({
      card: {
        id: 'benchmarkCases',
        providerId: 'benchmark.cases',
        labelKey: 'metrics.benchmarkCases',
        detailKey: 'metricsDetail.benchmarkCases',
        value: validated.benchmarkCaseCount,
        unitKey: 'units.cases',
        trace: {
          source: 'benchmarks',
          rule: 'Extract benchmark-like function declarations/macros with regex patterns.',
          paths: normalizedPaths.benchmarkPaths
        }
      }
    })
  },
  {
    id: 'test.files',
    produce: ({ validated, normalizedPaths }) => ({
      card: {
        id: 'testFiles',
        providerId: 'test.files',
        labelKey: 'metrics.testFiles',
        detailKey: 'metricsDetail.testFiles',
        value: validated.testFileCount,
        unitKey: 'units.files',
        trace: {
          source: 'tests',
          rule: 'Count readable .rs files in /tests.',
          paths: normalizedPaths.testPaths
        }
      }
    })
  },
  {
    id: 'test.cases',
    produce: ({ validated, normalizedPaths }) => ({
      card: {
        id: 'testCases',
        providerId: 'test.cases',
        labelKey: 'metrics.testCases',
        detailKey: 'metricsDetail.testCases',
        value: validated.testCaseCount,
        unitKey: 'units.cases',
        trace: {
          source: 'tests',
          rule: 'Extract #[test] function names with regex patterns.',
          paths: normalizedPaths.testPaths
        }
      }
    })
  },
  {
    id: 'test.assertions.avg',
    produce: ({ validated, normalizedPaths }) => ({
      card: {
        id: 'avgAssertionsPerTest',
        providerId: 'test.assertions.avg',
        labelKey: 'metrics.assertionsPerTest',
        detailKey: 'metricsDetail.assertionsPerTest',
        value: average(validated.testAssertions, validated.testCaseCount),
        unitKey: 'units.average',
        trace: {
          source: 'computed',
          rule: 'testAssertions / max(testCases, 1)',
          paths: normalizedPaths.testPaths
        }
      }
    })
  },
  {
    id: 'quality.phi.ratio',
    produce: ({ validated, normalizedPaths }) => ({
      card: {
        id: 'qualityPhiRatio',
        providerId: 'quality.phi.ratio',
        labelKey: 'metrics.qualityPhiRatio',
        detailKey: 'metricsDetail.qualityPhiRatio',
        value: Number(((validated.testCaseCount / Math.max(validated.benchmarkCaseCount, 1)) * PHI).toFixed(2)),
        unitKey: 'units.phi',
        trace: {
          source: 'computed',
          rule: '(testCases / max(benchmarkCases, 1)) * φ',
          paths: [...normalizedPaths.benchmarkPaths, ...normalizedPaths.testPaths]
        }
      }
    })
  },
  {
    id: 'benchmark.assertions',
    produce: ({ validated, normalizedPaths }) => ({
      card: {
        id: 'benchmarkAssertions',
        providerId: 'benchmark.assertions',
        labelKey: 'metrics.benchmarkAssertions',
        detailKey: 'metricsDetail.benchmarkAssertions',
        value: validated.benchmarkAssertions,
        unitKey: 'units.checks',
        trace: {
          source: 'benchmarks',
          rule: 'Count assertion macros in benchmark files.',
          paths: normalizedPaths.benchmarkPaths
        }
      }
    })
  }
];
