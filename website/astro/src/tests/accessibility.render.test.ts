import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';

const pagePath = path.resolve(import.meta.dirname, '../pages/index.astro');
const pageSource = fs.readFileSync(pagePath, 'utf8');

describe('accessibility and semantic rendering guards', () => {
  it('keeps skip link and main landmark wiring', () => {
    expect(pageSource).toContain('class="skip-link" href="#main-content"');
    expect(pageSource).toContain('<main id="main-content"');
    expect(pageSource).toContain('tabindex="-1"');
  });

  it('keeps calm warning semantics and disclosure structure', () => {
    expect(pageSource).toContain('role="status" aria-live="polite"');
    expect(pageSource).toContain('<ProvenancePanel');
  });

  it('keeps explicit section navigation label for assistive context', () => {
    expect(pageSource).toContain('aria-label={dict.accessibility.sectionNavLabel}');
  });
});
