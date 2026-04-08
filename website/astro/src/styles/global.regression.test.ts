import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';

const stylePath = path.resolve(import.meta.dirname, 'global.css');
const styleSource = fs.readFileSync(stylePath, 'utf8');

describe('global style regression guard', () => {
  it('keeps edge-case media queries for very small and very large viewports', () => {
    expect(styleSource).toContain('@media (max-width: 22rem)');
    expect(styleSource).toContain('@media (min-width: 110rem)');
  });

  it('keeps reduced-motion support and coarse-pointer handling', () => {
    expect(styleSource).toContain('@media (prefers-reduced-motion: reduce)');
    expect(styleSource).toContain('@media (hover: none), (pointer: coarse)');
  });
});
