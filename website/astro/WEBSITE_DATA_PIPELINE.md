# Website Data Pipeline

## Purpose

Generate landing metrics from read-only repository source inputs with explicit provenance.

## Stages

1. **Source read** (`source-reader.ts`)
   - reads `benchmarks/*.rs` and `tests/*.rs`
   - emits read issues (missing/unreadable)

2. **Extraction** (`extract-signals.ts`)
   - extracts benchmark/test names with regex patterns
   - counts assertion macros

3. **Validation** (`validate-signals.ts`)
   - checks for empty/malformed situations
   - emits warnings while preserving safe fallback values

4. **Path normalization** (`path-normalization.ts`)
   - converts absolute paths into stable repo-relative trace paths
   - emits unresolved marker when out-of-root

5. **Composition via registry** (`compose-metrics.ts` + registry)
   - runs all metric providers
   - aggregates metric cards, traces, and provider issues

## Metric Registry

Providers are defined in `src/data/registry/providers.ts` and executed by `metric-registry.ts`.

Provider output answers:
- where metric came from (`providerId` + trace source)
- how metric was computed (`trace.rule`)
- which files were involved (`trace.paths`)
- what failed (`issues` with provider context)

## Current Real Metrics

- benchmark files (count)
- benchmark cases (parsed)
- test files (count)
- test cases (parsed)
- benchmark assertions (count)
- assertions per test (computed)
- quality phi ratio (computed)

No fake values are emitted when inputs are missing.

## Tests

Vitest tests cover:
- extraction behavior (`extract-signals.test.ts`)
- validation warnings and count consistency (`validate-signals.test.ts`)
- registry output/provenance shape (`metric-registry.test.ts`)
- composition integration (`compose-metrics.test.ts`)
- path normalization (`path-normalization.test.ts`)
- locale fallback behavior (`i18n/index.test.ts`)

## Extension Guide

To add a new metric domain:
1. extend source read config only if needed
2. add extractor/validator changes for new signals
3. implement a new provider in `providers.ts`
4. add tests for provider rules and failure cases
5. add localized labels/details/units
