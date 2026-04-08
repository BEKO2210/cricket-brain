import fs from 'node:fs';
import path from 'node:path';
import { readOnlySourceDirectories } from '@/config/boundary';
import type { PipelineIssue, RawRustSource } from '@/types/metrics';

type SourceReadResult = {
  benchmarkFiles: RawRustSource[];
  testFiles: RawRustSource[];
  issues: PipelineIssue[];
};

const readDirectoryRustFiles = (dirPath: string): { files: RawRustSource[]; issues: PipelineIssue[] } => {
  const issues: PipelineIssue[] = [];

  if (!fs.existsSync(dirPath)) {
    issues.push({ level: 'error', message: 'Source directory does not exist.', context: dirPath });
    return { files: [], issues };
  }

  let filenames: string[] = [];
  try {
    filenames = fs.readdirSync(dirPath).filter((name: string) => name.endsWith('.rs'));
  } catch (error) {
    issues.push({
      level: 'error',
      message: 'Unable to read source directory.',
      context: `${dirPath}: ${error instanceof Error ? error.message : 'unknown error'}`
    });
    return { files: [], issues };
  }

  const files = filenames.flatMap((filename: string) => {
    const absolutePath = path.join(dirPath, filename);
    try {
      return [
        {
          path: absolutePath,
          filename,
          content: fs.readFileSync(absolutePath, 'utf-8')
        }
      ];
    } catch (error) {
      issues.push({
        level: 'warning',
        message: 'Unable to read source file.',
        context: `${absolutePath}: ${error instanceof Error ? error.message : 'unknown error'}`
      });
      return [];
    }
  });

  return { files, issues };
};

export const readProjectSources = (): SourceReadResult => {
  const benchmarkRead = readDirectoryRustFiles(readOnlySourceDirectories.benchmarks);
  const testRead = readDirectoryRustFiles(readOnlySourceDirectories.tests);

  return {
    benchmarkFiles: benchmarkRead.files,
    testFiles: testRead.files,
    issues: [...benchmarkRead.issues, ...testRead.issues]
  };
};
