import { repositoryRoot } from '@/config/boundary';
import { composeLandingMetrics } from '@/data/pipeline/compose-metrics';
import { extractSignals } from '@/data/pipeline/extract-signals';
import { readProjectSources } from '@/data/pipeline/source-reader';
import { validateSignals } from '@/data/pipeline/validate-signals';
import type { LandingMetrics } from '@/types/metrics';
import { normalizeToRepoRelativePath } from '@/utils/path-normalization';

export const getLandingMetrics = (): LandingMetrics => {
  const sourceRead = readProjectSources();
  const extracted = extractSignals(sourceRead.benchmarkFiles, sourceRead.testFiles);
  const validated = validateSignals(extracted);

  return composeLandingMetrics(
    validated.validated,
    extracted.benchmarkFunctionNames,
    extracted.testFunctionNames,
    {
      benchmarkPaths: extracted.benchmarkFiles.map((file) => normalizeToRepoRelativePath(file.path, repositoryRoot)),
      testPaths: extracted.testFiles.map((file) => normalizeToRepoRelativePath(file.path, repositoryRoot))
    },
    [...sourceRead.issues, ...validated.issues]
  );
};
