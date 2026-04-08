export const en = {
  meta: {
    title: 'Cricket Brain | Precision Intelligence Engine',
    description:
      'Structured product landing page built from real benchmark and test source signals with traceable provenance.'
  },
  hero: {
    kicker: 'Adaptive Neural Runtime',
    title: 'Repository signals, structured into product-grade evidence.',
    subtitle:
      'This page is organized as a technical narrative: what is measured, how it is measured, and how those signals are exposed for decision-making.',
    ctaPrimary: 'View Live Proof',
    ctaSecondary: 'Inspect Trust Layer',
    pillSource: 'Source-derived metrics',
    pillTyped: 'Typed pipeline',
    pillReadonly: 'Read-only project boundary'
  },
  accessibility: {
    skipToMain: 'Skip to main content',
    sectionNavLabel: 'Section navigation'
  },
  nav: {
    proof: 'Proof',
    highlights: 'Highlights',
    visualization: 'Visualization',
    principles: 'Principles',
    evidence: 'Evidence',
    extensibility: 'Extensibility'
  },
  sections: {
    proofKicker: 'Core Proof',
    proofTitle: 'Live repository metrics',
    proofSummary: 'Metrics are sourced from read-only project files or explicit computed rules.',
    highlightsKicker: 'Technical Highlights',
    highlightsTitle: 'How the website pipeline is engineered',
    highlightsSummary: 'A staged pipeline preserves traceability and resilience with minimal noise.',
    visualizationKicker: 'Visualization Context',
    visualizationTitle: 'Phi-based metric field with overlap safeguards',
    visualizationSummary:
      'The spiral exists to compare relationships quickly while preserving readable spacing on mobile and desktop.',
    principlesKicker: 'Engineering Principles',
    principlesTitle: 'Structural quality rules guiding the site',
    principlesSummary: 'Boundary control, typed contracts, and deterministic fallbacks keep the system credible.',
    evidenceKicker: 'Capability Signals',
    evidenceTitle: 'Parsed benchmark and test coverage map',
    evidenceSummary: 'Names below come directly from source parsing results and update with repository changes.',
    extensibilityKicker: 'Future Readiness',
    extensibilityTitle: 'Designed for safe expansion',
    extensibilitySummary: 'Registry providers and locale loaders make controlled growth straightforward.',
    trustKicker: 'Trust & Provenance',
    trustTitle: 'Trace rules and fallback reporting',
    trustSummary: 'Each metric exposes source rules and path context, with calm warning and fallback framing.',
    warningsTitle: 'Data pipeline warnings',
    warningsSummary: '{count} active warning entries from current pipeline run.',
    benchmarkList: 'Benchmark functions detected',
    testList: 'Test functions detected'
  },
  bullets: {
    highlight1: 'Read-only source access from /benchmarks and /tests through a dedicated boundary layer.',
    highlight2: 'Staged pipeline: read → extract → validate → compose via metric registry.',
    highlight3: 'Trace metadata for every metric card: source, rule, and file path context.',
    principle1: 'No fabricated values: unavailable inputs become warnings, never invented metrics.',
    principle2: 'Provider failures are isolated so one metric issue does not collapse the full page.',
    principle3: 'Composition rhythm uses φ for spacing and comparative density only where useful.',
    extensibility1: 'Add new metric providers without rewiring section markup.',
    extensibility2: 'Locale loader supports controlled language expansion with predictable fallback.',
    extensibility3: 'Website-only CI validates checks, tests, and build independently from core flows.'
  },
  status: {
    overlapSafety: 'Overlap safety',
    overlapPass: 'No card overlap detected',
    overlapAdjusted: 'Layout adjusted to prevent overlap',
    updatedAt: 'Updated'
  },
  units: {
    files: 'files',
    cases: 'cases',
    average: 'avg',
    phi: 'phi',
    checks: 'checks'
  },
  metrics: {
    benchmarkFiles: 'Benchmark Files',
    benchmarkCases: 'Benchmark Cases',
    testFiles: 'Test Files',
    testCases: 'Test Cases',
    assertionsPerTest: 'Assertions / Test',
    qualityPhiRatio: 'Quality Ratio (φ)',
    benchmarkAssertions: 'Benchmark Assertions'
  },
  metricsDetail: {
    benchmarkFiles: 'Count of readable benchmark source files',
    benchmarkCases: 'Parsed benchmark declarations from benchmark files',
    testFiles: 'Count of readable test source files',
    testCases: 'Parsed #[test] functions from test files',
    assertionsPerTest: 'Computed as test assertions divided by parsed test cases',
    qualityPhiRatio: 'Computed as (test cases / benchmark cases) × φ',
    benchmarkAssertions: 'Assertion macro count inside benchmark source files'
  },
  provenance: {
    title: 'Metric provenance summary',
    summaryLabel: 'Source distribution',
    empty: 'No provenance entries available from current pipeline output.'
  },
  footer: {
    text: 'Structured for factual communication first; visual refinement can layer on this architecture.'
  },
  fallback: {
    noEntries: 'No entries available from source parsing.',
    noWarnings: 'No data pipeline warnings detected.'
  }
};

export type Dictionary = typeof en;
