import type { ExtractedSignals, PipelineIssue, ValidatedSignals } from '@/types/metrics';

type ValidationResult = {
  validated: ValidatedSignals;
  issues: PipelineIssue[];
};

export const validateSignals = (signals: ExtractedSignals): ValidationResult => {
  const issues: PipelineIssue[] = [];

  if (!signals.benchmarkFiles.length) {
    issues.push({ level: 'warning', message: 'No benchmark files were available for parsing.' });
  }

  if (!signals.testFiles.length) {
    issues.push({ level: 'warning', message: 'No test files were available for parsing.' });
  }

  if (signals.testFunctionNames.length && signals.testAssertions === 0) {
    issues.push({
      level: 'warning',
      message: 'Test functions were detected but no assertion macros were found.'
    });
  }

  const validated: ValidatedSignals = {
    benchmarkFileCount: signals.benchmarkFiles.length,
    testFileCount: signals.testFiles.length,
    benchmarkCaseCount: signals.benchmarkFunctionNames.length,
    testCaseCount: signals.testFunctionNames.length,
    benchmarkAssertions: signals.benchmarkAssertions,
    testAssertions: signals.testAssertions
  };

  return { validated, issues };
};
