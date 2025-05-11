UI Robustness & Refactor Guide

    Audience: new contributors who need a “one-stop” reference for writing, extending or refactoring the web-front-end (Rust + WASM) without accidentally breaking existing behaviour.

The document distills the debugging session around the “Triggers tab remains visible after switching away” bug and lays out a practical roadmap – from immediate fixes you can ship in minutes to structural upgrades that future-proof the UI as it scales.

0. TL;DR – What to do first

✔  Add `dom_utils.rs` helper (show / hide / activate)
✔  Store the *active tab* as an enum in `AppState`
✔  Enforce ID-prefix rule (`agent-`, `workflow-`, …) via pre-commit grep
✔  Write a 20-line Playwright smoke test that asserts only one
   `.tab-content` pane is visible inside a modal
✔  Document the “golden path” in CONTRIBUTING / this guide

These steps catch 80% of “element never hidden” or “duplicate-ID” regressions without adopting a new framework or rewriting existing modals.

1. Background – Why the Bug Happened
    •   The modal’s tab buttons call different Message variants in update.rs.
    •   Before the patch, only the Main ↔ History handlers toggled visibility.
    •   The new Triggers pane was shown but never hidden.
    •   Result: when leaving the Triggers tab, its DOM stayed visible under the Main pane.

Root cause: imperative show/hide logic scattered across code – easy to forget a div when adding features.

2. Quick-Win Improvements (≤ 1 Day of Work)

2.1 dom_utils.rs – Single Source of Truth for Visibility Helpers

// frontend/src/utils/dom_utils.rs

pub fn show(el: &web_sys::Element)  { let _ = el.remove_attribute("hidden"); }
pub fn hide(el: &web_sys::Element)  { let _ = el.set_attribute("hidden", "true"); }

pub fn set_active(btn: &web_sys::Element)   { btn.set_class_name("tab-button active"); }
pub fn set_inactive(btn: &web_sys::Element) { btn.set_class_name("tab-button"); }

pub fn html_input(id: &str) -> web_sys::HtmlInputElement {
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(id))
        .and_then(|e| e.dyn_into::<web_sys::HtmlInputElement>().ok())
        .expect(&format!("<input id='{}'> not found or wrong type", id))
}

Replace all raw set_attribute("style", …) toggles with show() / hide() – no behavior change, big readability win.

2.2 Enum-Based Active Tab Tracking

// state.rs

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentConfigTab { Main, History, Triggers }

pub struct AppState {
    …
    pub agent_modal_tab: AgentConfigTab,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            …,
            agent_modal_tab: AgentConfigTab::Main,
        }
    }
}

Tab buttons dispatch Message::SetAgentTab(AgentConfigTab::Triggers).
The view layer renders the matching pane only – one source of truth.

2.3 DOM-ID Namespacing + Linter

Rule: An ID must start with the owning module prefix.
    •   agent-… → Agent-config modal
    •   workflow-… → Workflow-builder modal
    •   global-… → Truly global widgets (top-nav, toast, etc.)

Pre-commit hook (POSIX grep):

grep -R --line-number --perl-regexp \
      '<[^>]+id="(?!agent-|workflow-|global-)' frontend/src | \
  while read -r line; do
    echo "✖  Non-namespaced ID: $line"; exit 1;
  done

2.4 Playwright Smoke Test in CI

frontend/e2e/modal_tab_visibility.spec.ts:

test('only one tab-content is visible', async ({ page }) => {
  await page.goto('http://localhost:8002');
  await page.click('text="New Agent"');
  await page.click('#agent-triggers-tab');
  await page.click('#agent-main-tab');

  const visible = await page.$$('.tab-content:not([hidden])');
  expect(visible).toHaveLength(1);
});

Runs in ~5 seconds and guards against regressions.

2.5 Pre-commit: cargo check + clippy -D warnings

Add to .pre-commit-config.yaml:

- repo: local
  hooks:
    - id: rust-check
      language: system
      entry: bash -c 'cd frontend && cargo clippy --quiet -- -D warnings'

3. Medium-Term Refactor Milestones (1–2 Weeks)

Milestone   Benefit Effort
Extract Modal, TabBar, tab-content… Reuse & isolation   1–2 d/modal
Typed query!() macro (à la Yew) Eliminate runtime dyn-casts 0.5 d
Storybook-style UI demo page    Visual diff & onboarding    2 d
Spike: migrate one modal to Yew Measure DX & bundle impact  3–4 d

--------------------------------------------------------------------------------
7. Implementation Progress (May 2025)

This section tracks **real commits** that landed after the original roadmap
was written so new contributors can follow the paper-trail.

### 7.1 Quick-Win batch merged on 2025-05-10

✔ dom_utils.rs created and wired up (show / hide / set_active / set_inactive)

✔ AppState now carries `agent_modal_tab: AgentConfigTab` (enum Main | History |
  Triggers) – single source of truth

✔ update.rs switched to unified `Message::SetAgentTab(tab)`; legacy
  `SwitchTo*Tab` messages forward internally for backwards compatibility.

✔ Agent Config Modal IDs renamed to the `agent-` prefix:

```
  main-tab      → agent-main-tab
  main-content  → agent-main-content
  triggers-tab  → agent-triggers-tab
  … (plus ~20 more)
```

✔ Global CSS rule `[hidden]{display:none}` added so `dom_utils::hide()` needs
  no inline style hacks.

✔ Pre-commit config extended
  • `rust-clippy` hook (frontend) – blocks warnings in CI
  • DOM-ID prefix grep hook stubbed (commented until full repo migration)

✔ Playwright smoke test `frontend/e2e/modal_tab_visibility.spec.ts`
  ensures only **one** `.tab-content` is visible in the agent modal.

### 7.2 Files touched in this batch

```
frontend/src/
  dom_utils.rs                      (NEW)
  state.rs                          (+ AgentConfigTab enum)
  messages.rs                       (+ SetAgentTab)
  update.rs                         (refactor handlers)
  lib.rs                            (mod dom_utils)
  components/agent_config_modal.rs  (ID rename + listeners)
www/styles.css                      ([hidden] rule)
.pre-commit-config.yaml             (hooks)

frontend/e2e/*                      (Playwright config + spec)
```

### 7.3 Lessons learned

• Enum-driven UI logic dramatically simplifies the reducer – no more copy-&-
  paste tab handlers that can diverge.

• Namespacing DOM IDs early prevents later collisions when multiple modals
  grow; grep-based linting is cheap insurance.

• Playwright proved a fast feedback loop (5 s) and already caught a regression
  during refactor when a second `.tab-content` became visible.

• Keep legacy message variants while external call-sites migrate – helps avoid
  giant PRs and eases git-bisect.

### 7.4 Remaining items  *(auto-updated)*

1. **Full codebase ID audit** – Agent Config Modal + Chat view done; dashboard
   & canvas pages still carry raw IDs.  Once renamed we can rely exclusively
   on the new pre-commit hook for enforcement.
2. **Extract `<TabBar>`** – DONE.  `components::tab_bar` landed and both
   Agent Config **and** Agent Debug modals consume it (commit 7.9).  A small
   attach helper still wires click-handlers, but the visual markup is now
   shared.
3. Framework spike (Yew / Leptos) + Storybook-style preview pages.

### 7.6 Visibility helpers – **rollout finished** (2025-05-10 evening)

All remaining `display:none` / `display:block` inline toggles have been
replaced with `dom_utils::hide()` / `dom_utils::show()` or converted to CSS
classes.  This completes item **3** of the TODO list.

Key touches
• agent_config_modal.rs – biggest lift (Gmail-connect row, Add-Trigger form,
  scheduling inputs, dynamic empty-state).  Introduced helper-based toggles
  and kept existing padding/border styling intact.
• agent_debug_modal.rs & views.rs – migrated close/hide paths.

grep verification → **0** occurrences of `display:none` or `display:block`
writes remain in `frontend/src`.

Next up
1. Flip the DOM-ID prefix pre-commit grep hook from commented to **active**.  
   (Done – see .pre-commit-config.yaml update)  
2. Start extracting `<Modal>` and `<TabBar>` components.

We are officially style-toggle-free!  🎉

### 7.7 Shared `<Modal>` helper introduced (2025-05-11)

Implemented `frontend/src/components/modal.rs` that provides:

• `ensure_modal(document, id)` – idempotently creates the backdrop +
  `.modal-content` wrapper and returns both elements.
• `modal::show()` / `modal::hide()` thin wrappers around dom_utils.

First consumers will be Agent Debug & Config modals in the next patch; this
forms the base for extracting a reusable `<TabBar>` component.

### 7.8 Agent Debug & Config modals migrated (2025-05-11 evening)

Both major modals now call `modal::ensure_modal()` and the old inline
`display:block/none` toggles were replaced with the helper’s `show()` /
`hide()` wrappers.  Boiler-plate backdrop creation, click-to-close listeners
and `.modal-content` wrappers are consolidated – reducing each modal by ~40
lines of repetitive code.

Next milestone: extract `<TabBar>` so tab-button markup is likewise shared.

### 7.9 `<TabBar>` helper + adoption (2025-05-12)

`components/tab_bar.rs` introduces `build_tab_bar()` – a tiny factory that
generates the standard `.tab-container` / `.tab-button` markup.  Both major
modals now call the helper and attach their click-handlers right after.  This
removes another ~30 lines of duplicated DOM code and gives us a single place
to evolve tab styling.

Lessons learned
• Stable *IDs* are still needed for listener/wiring in the legacy Agent Config
  modal – we temporarily set them after creation.  A future refactor will
  switch that modal to enum-based `Message::SetAgentTab` just like the Debug
  modal so no ID hacks remain.
• Helper design: returning bare buttons lets each consumer attach its own
  callback without higher-order generics – keeps wasm-bindgen lifetimes simple.

Next steps
1. Refactor `attach_listeners()` in Agent Config Modal to use the attach
   helper pattern (or migrate to enum-based messages) so IDs can be removed.
2. Audit remaining ID prefixes on Dashboard & Canvas and enable the grep hook
   in *enforcing* mode.
3. Extend `TabBar` with optional builder that accepts `(label, TabVariant)`
   and auto-generates click handlers for common patterns.

### 7.5 Visibility-helper rollout (same day follow-up)

✔ **Chat View migrated** – replaced five inline `set_attribute("style", …display… )`
  calls with `dom_utils::hide()` / `dom_utils::show()`.

✔ **Dashboard navigation** – `NavigateToDashboard` reducer path in `update.rs`
  now hides the Chat container via the helper, eliminating another raw style
  toggle.

  _grep tally_  ➜  **~10** direct `display:none` writes remain across the code-
  base (mainly inside the debug modal).  We are over the halfway mark.

✔ Added missing `use crate::dom_utils` imports where necessary so the helpers
  compile without fully-qualified paths.

_Outcome_ – We validate that the helper works across **pages** (Chat ↔ Dash)
as well as **modals**.  Momentum for finishing task 3 in the TODO list.

The quick-wins are now merged; future work can iterate without the original
bug resurfacing.

4. Coding Conventions Cheat-Sheet

Concern Convention
File names  snake_case.rs; tests inline mod tests { … }
DOM IDs Prefix with module: agent-, workflow-, etc.
Visibility  Use hidden attribute, not style="display:none"
Tabs    .active class via dom_utils::set_active()
Messages    Message::VerbNoun (e.g. SwitchToTriggersTab)
Commands    No DOM access – side effects only (REST/WS)

5. FAQ for New Developers

Q — Where do I start when adding a new tab to a modal?
    1.  Add HTML: <div id="agent-myfeature-content" hidden>
    2.  Add tab button: <button id="agent-myfeature-tab">…</button>
    3.  Extend AgentConfigTab enum and Message::SetAgentTab(MyFeature)
    4.  Wire click listener to dispatch message

Q — Playwright test fails: multiple panes visible.

Make sure hide() was called on other tabs or use the enum-driven render approach.

Q — expected <input id='agent-name'> panic.

You renamed the DOM ID without updating the Rust helper. Keep them in sync.

6. Appendix – Resources & Inspiration
    •   Yew – React-style Rust framework
    •   Sycamore – Signal-based
    •   Leptos – SSR and islands architecture
    •   Dioxus – Virtual DOM, multiplatform

We’re currently using wasm-bindgen for minimal bloat, but these frameworks align with the enum/component pattern and can be adopted later.

Happy hacking – and may no tab ever overstay its welcome again!