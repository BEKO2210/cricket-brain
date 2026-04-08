import { describe, expect, it } from 'vitest';
import { normalizeToRepoRelativePath } from '@/utils/path-normalization';

describe('normalizeToRepoRelativePath', () => {
  it('returns stable repo-relative paths', () => {
    const result = normalizeToRepoRelativePath('/repo/tests/sample.rs', '/repo');
    expect(result).toBe('tests/sample.rs');
  });

  it('returns unresolved marker for paths outside repository root', () => {
    const result = normalizeToRepoRelativePath('/other/location/file.rs', '/repo');
    expect(result.startsWith('[unresolved]')).toBe(true);
  });
});
