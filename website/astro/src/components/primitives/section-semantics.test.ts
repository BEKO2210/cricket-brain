import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';

const blockPath = path.resolve(import.meta.dirname, 'SectionBlock.astro');
const headerPath = path.resolve(import.meta.dirname, 'SectionHeader.astro');
const blockSource = fs.readFileSync(blockPath, 'utf8');
const headerSource = fs.readFileSync(headerPath, 'utf8');

describe('section semantic guards', () => {
  it('keeps aria-labelledby wiring between section and h2', () => {
    expect(blockSource).toContain('aria-labelledby={headingId}');
    expect(headerSource).toContain('<h2 id={headingId}>');
  });
});
