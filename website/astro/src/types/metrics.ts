export type SourceId = 'benchmarks' | 'tests' | 'computed';

export type SourceTrace = {
  source: SourceId;
  rule: string;
  paths: string[];
};

export type PipelineIssue = {
  level: 'warning' | 'error';
  message: string;
  context?: string;
};

export type RawRustSource = {
  path: string;
  filename: string;
  content: string;
};

export type ExtractedSignals = {
  benchmarkFiles: RawRustSource[];
  testFiles: RawRustSource[];
  benchmarkFunctionNames: string[];
  testFunctionNames: string[];
  benchmarkAssertions: number;
  testAssertions: number;
};

export type ValidatedSignals = {
  benchmarkFileCount: number;
  testFileCount: number;
  benchmarkCaseCount: number;
  testCaseCount: number;
  benchmarkAssertions: number;
  testAssertions: number;
};

export type MetricCard = {
  id: string;
  providerId: string;
  labelKey: string;
  detailKey: string;
  value: number;
  unitKey: string;
  trace: SourceTrace;
};

export type DisplayMetricCard = {
  id: string;
  label: string;
  detail: string;
  value: number;
  unit: string;
  traceHint?: string;
};

export type MetricProviderContext = {
  validated: ValidatedSignals;
  benchmarkNames: string[];
  testNames: string[];
  normalizedPaths: {
    benchmarkPaths: string[];
    testPaths: string[];
  };
};

export type MetricProvider = {
  id: string;
  produce: (context: MetricProviderContext) => { card?: MetricCard; issues?: PipelineIssue[] };
};

export type LandingMetrics = {
  updatedAt: string;
  cards: MetricCard[];
  benchmarkNames: string[];
  testNames: string[];
  issues: PipelineIssue[];
  traces: Record<string, SourceTrace>;
};
