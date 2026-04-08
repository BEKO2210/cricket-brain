# Website Workflow

## Purpose

`website-astro.yml` is a website-only CI workflow for the isolated Astro subsystem.

It validates only `website/astro/**` and does not alter or run core project workflows.

## Trigger Scope

Workflow triggers on:
- pushes affecting `website/astro/**`
- pull requests affecting `website/astro/**`
- changes to `.github/workflows/website-astro.yml`

## Commands Executed (within `website/astro`)

1. `npm ci`
2. `npm run check`
3. `npm run test:ci`
4. `npm run build`

## Isolation Guarantees

- Job `working-directory` is pinned to `website/astro`
- dependency cache is pinned to `website/astro/package-lock.json`
- no Rust/core project build steps are included

## Failure Behavior

The workflow fails if:
- Astro/type checks fail
- data pipeline tests fail
- locale tests fail
- build fails

## Extending Workflow Safely

To add more checks:
- keep commands scoped to `website/astro`
- avoid touching core project release/CI jobs
- prefer website npm scripts for consistency
