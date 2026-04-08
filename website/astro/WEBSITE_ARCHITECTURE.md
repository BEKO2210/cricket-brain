# Website Architecture (Astro v5)

## Scope and Boundary

Website code is isolated under `website/astro/**`.

- Writable scope: `website/astro/**`
- Read-only external inputs: `<repo>/benchmarks` and `<repo>/tests`
- Boundary config: `src/config/boundary.ts`

## Semantic Structure Decisions

- Root regions are explicit: skip-link, `main`, sectional landmarks, and footer
- Each content section uses `aria-labelledby` wired to its own `<h2>`
- In-page section nav has a descriptive `aria-label` and active-location semantics (`aria-current="location"`)
- Trust and warning content are semantically separated so users can distinguish provenance detail from runtime issues

## Heading and Reading Order

- Single page `<h1>` in hero
- Section titles are `<h2>` via shared `SectionHeader`
- Nested cards/panels use `<h3>` where needed for local grouping
- Visual ordering matches DOM reading order to preserve assistive-tech scan flow

## Focus, Keyboard, and Touch Conventions

- Visible `:focus-visible` ring across interactive controls
- Skip-link enables immediate keyboard jump to main content
- Disclosure summaries remain keyboard-native (`details/summary`) with visible focus treatment
- Coarse pointers do not depend on hover-only cues
- Coarse-pointer controls keep minimum target height and receive `:active` feedback

## Provenance Summary vs Detail

Trust information is intentionally two-layered:
- **Summary layer**: source-distribution chips (`benchmarks/tests/computed` counts) for instant scan
- **Detail layer**: `details/summary` per metric showing rule + representative path

Ordering is deterministic (`source` then `metricId`) to keep trust reading stable across runs.

## Warning and Fallback Framing

- Warning area includes calm summary count (`role="status"`, `aria-live="polite"`)
- Fallback empty states are explicit and intentional
- Styling is informative, not alarmist

## Contrast-Sensitive Matrix (Run 9)

Reviewed and validated states for:
- default/body text
- supporting text and section summaries
- trace source/path text
- warning summary + warning details
- CTA default/hover/focus/active
- nav default/active/focus
- disclosure summary/default/open/focus
- focus rings on tinted surfaces

Only verified weak points were adjusted (no redesign pass).

## Motion and Reduced Motion

- Shared motion tokens: `--dur-fast`, `--dur-mid`, `--ease-standard`
- Interaction motion remains subtle and utilitarian
- `prefers-reduced-motion` disables non-essential transitions/animations

## Responsive Edge Strategy

- Extra-small screens (`max-width: 22rem`): full-width CTA, wrapped pills, tighter shell
- Desktop (`>=72rem`): expanded rhythm and stronger hero anchoring
- Ultra-wide (`>=110rem`): max width cap avoids whitespace deserts

## Rendering-Level Test Scope

- `src/tests/accessibility.render.test.ts` validates skip-link/main wiring, warning semantics, and nav labeling
- `src/tests/index.regression.test.ts` validates critical sections, active-nav logic presence, and warning/no-warning branches
- `src/components/primitives/section-semantics.test.ts` validates `aria-labelledby` heading wiring
- `src/styles/global.regression.test.ts` validates responsive and reduced-motion/coarse-pointer media guards

These checks intentionally prioritize maintainability and practical confidence over brittle screenshot testing.

## Meaningful φ Influence

φ remains applied where composition benefits:
- spacing rhythm tokens (`--space-*`)
- metric computation (`qualityPhiRatio`)
- spiral placement geometry
