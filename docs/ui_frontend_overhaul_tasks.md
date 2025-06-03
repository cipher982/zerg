# UI & Front-End Overhaul – Task Tracker

This markdown file tracks the high-level work items for the upcoming visual overhaul of the Zerg web front-end.  Tasks are grouped by phase and organised as GitHub-style check-lists so they can serve directly as an issue description or project board source.

> Tip: Tick the `[ ]` boxes as you complete each item – GitHub, GitLab and many editors render them as interactive check-lists.

---

## Phase 0 – Scaffolding

- [x] Create **`frontend/www/tokens.css`** with design-token variables (colours, spacing, radii, shadows, transitions).
- [x] Import `tokens.css` at the top of **`frontend/www/styles.css`**.
- [x] Replace hard-coded colour / spacing values in `styles.css` with `var(--token)` references (primary-heavy blue values swapped).
- [ ] Add a tiny helper macro `css_var!()` (or similar) for ergonomic inline styles in the Rust/Yew code-base.
- [ ] Run `trunk build` (or existing build script) and visually verify there are no regressions. *(pending manual review)*

## Phase 1 – Quick-Win Theming

- [x] Swap legacy blue `#3498db` highlights for `var(--primary)`.
- [x] Apply primary/secondary gradient to header, tab-bar, and status-bar backgrounds.
- [x] Update global font stack & root font-size to match branding draft.
- [ ] Run an automated contrast check (axe-core or similar) – fix any AA failures.

## Phase 2 – Component Polish

### Buttons
- [x] Implement reusable Button component variants (primary / secondary / ghost).
- [x] Add size modifiers (sm / md / lg).
- [x] Animate hover/active states using `var(--transition-normal)`.

### Agent Pill & Shelf
- [x] Apply `--radius-lg` and token-based shadows.
- [ ] Harmonise drag-and-drop “ghost” styling with new palette.

### Modal & Form Elements
- [ ] Standardise spacing, typography and focus-ring colour.

### Toast / Alert System
- [ ] Create Toast component with success / warning / error themes.

## Phase 3 – Layout & Responsiveness

- [ ] Introduce `layout.css` (or util classes) for common flex/grid patterns.
- [ ] Refactor canvas view to use modern `clamp()` sizing.
- [ ] Collapse agent shelf into off-canvas drawer on < 768 px viewports.

## Phase 4 – Delight Layer

- [ ] Add particle background effect to login/landing screen.
- [ ] Add micro-interactions (button scale, tab ripple, modal fade).
- [ ] Integrate SVG icon set (Lucide/Feather) using `currentColor` for easy theming.

## Phase 5 – Documentation & Handoff

- [ ] Generate Storybook-style **Component Gallery** route under `/dev/components`.
- [ ] Write `docs/design_system.md` explaining tokens, components, and usage rules.
- [ ] Update root **README.md** with design-system section and contribution guide.
- [ ] Establish visual-regression baseline screenshots and add to CI.

## Tooling / Quality Gates (parallel tasks)

- [ ] Add `stylelint` & `prettier-css` to **pre-commit** config.
- [ ] Integrate `axe-playwright` for automated accessibility checks.
- [ ] Configure visual-regression tests (Playwright + pixel-diff) for critical pages.

---

### How to use this file

1. **Single source of truth** – Keep this list updated instead of scattered TODOs.
2. **PR integration** – Reference the relevant check-box line in pull-request descriptions so merges automatically tick items.
3. **Review cadence** – During planning meetings, skim this file to pick next priorities.

Happy shipping! 🚀
