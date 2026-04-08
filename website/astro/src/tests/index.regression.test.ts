import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';

const pagePath = path.resolve(import.meta.dirname, '../pages/index.astro');
const pageSource = fs.readFileSync(pagePath, 'utf8');

describe('index page structural regression guard', () => {
  it('keeps critical trust/proof/visual sections present', () => {
    expect(pageSource).toContain('id="proof"');
    expect(pageSource).toContain('id="visualization"');
    expect(pageSource).toContain('id="trust"');
  });

  it('keeps in-page nav links and scrollspy hook classes', () => {
    expect(pageSource).toContain('class="hero-nav-link" href="#proof"');
    expect(pageSource).toContain('class="hero-nav-link" href="#visualization"');
    expect(pageSource).toContain('IntersectionObserver');
    expect(pageSource).toContain("link.setAttribute('aria-current', 'location')");
    expect(pageSource).toContain('link.removeAttribute(\'aria-current\')');
  });

  it('keeps both warning and no-warning rendering branches', () => {
    expect(pageSource).toContain('metrics.issues.length ? (');
    expect(pageSource).toContain('dict.fallback.noWarnings');
  });
});
