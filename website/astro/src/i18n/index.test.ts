import { describe, expect, it } from 'vitest';
import { getDictionary, loadDictionary, t } from '@/i18n';

describe('i18n locale foundation', () => {
  it('falls back to english for unsupported locale', async () => {
    const dict = await loadDictionary('de');
    expect(dict.meta.title).toContain('Cricket Brain');
  });

  it('returns key when translation key is missing', () => {
    const dict = getDictionary('en');
    expect(t(dict, 'missing.key')).toBe('missing.key');
  });
});
