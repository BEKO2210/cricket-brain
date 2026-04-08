import path from 'node:path';

export const normalizeToRepoRelativePath = (absolutePath: string, repositoryRoot: string): string => {
  const resolvedRoot = path.resolve(repositoryRoot);
  const resolvedPath = path.resolve(absolutePath);

  const relativePath = path.relative(resolvedRoot, resolvedPath);
  if (!relativePath || relativePath.startsWith('..')) {
    return `[unresolved] ${absolutePath}`;
  }

  return relativePath.split(path.sep).join('/');
};
