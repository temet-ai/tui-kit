---
title: "Thread: tui-kit-crate-integrations"
summary: Thread started.
type: Thread
status: review
created: 2026-08-21
updated: 2026-08-23
thread: tui-kit-crate-integrations
last_session: 2026-08-23 (session unknown) @ fcfdf51
acceptance_criteria: ## Acceptance criteria
---

## State of play
Where we are: Thread started.
Next: (1) Define first step.
Dependencies: none.
Branch: none.
Worktree: none.
Open questions: none.
Decisions: none.
Blockers: none.

## Acceptance criteria
<acceptance-criteria>
1. Deps upgraded: ratatui 0.30.x + ratatui-image 11.x, crate-root re-exports intact; cargo build, cargo clippy --all-targets, and full cargo test green — run by orchestrator, output shown.
2. Ratzilla browser demo: new workspace member compiling to wasm32-unknown-unknown that renders the temet card gallery in-browser (cell/halfblocks rendering of card images acceptable); build verified by orchestrator.
3. tui-shader behind a non-default cargo feature (shader) with a runnable example; default cargo build pulls no wgpu.
4. notify-driven hot reload: gallery gains a watch mode that re-rasterizes/re-renders when theme/SVG inputs change; demonstrated by scripted check or test.
5. Temet visual spec unregressed: no color literals outside theme.rs/svg.rs (fixtures excepted); existing tests pass unmodified, any test change disclosed.
6. Non-goals: no crates.io publish, no web-demo deployment, no rat-widget/tui-widgets adoption, no git commits unless asked.
Target: /Users/dawiddutoit/projects/tui-kit @ main working tree (uncommitted changes present, repo at fcfdf51).
</acceptance-criteria>

## Work units
| id | what | owner | tier | status | allowlist | verify | evidence | rounds | handle |
|---|---|---|---|---|---|---|---|---|---|
| recon | 4-lane recon: migration deltas, ratzilla, tui-shader+notify, local API inventory | workflow | R2/R1 | verified | (read-only) | n/a | payloads recovered from journal (3 web lanes returned placeholders after StructuredOutput validation failures; real content in agent-*.jsonl failed attempts); ratatui::crossterm survival confirmed by orchestrator via docs.rs | 0 | wf_aafc04a7-124 |
| upgrade | ratatui 0.30 + ratatui-image 11 across crate; owns ALL Cargo.toml edits: input/shader features, tui-shader =0.0.10 optional, notify dev-deps, shader stubs+[[example]] | sonnet high | R3 (S2) | verified | Cargo.toml, src/*, examples/*, tests/* | independent R2 + orchestrator rerun | verify agent PASS (58 tests, only test change = mechanical (8,16).into() in stands_up.rs); orchestrator rerun: build/clippy/test/ndf all green | 0 | wf_7b830d6a-b9a |
| web-demo | ratzilla 0.3.1 browser gallery, standalone crate web-gallery/ (own [workspace], path dep on portable core, halfblocks cells; [patch.crates-io] rustix → vendor/rustix-wasm-shim scoped inside web-gallery) | opus high | R4 (S3+S4 wasm) | verified | web-gallery/** (+rustup target add wasm32 authorized) | independent wasm build + main.rs review + orchestrator rerun | verify agent PASS (clean 40s wasm rebuild; main.rs uses real CardGrid/Theme::temet()/Picker::halfblocks()); orchestrator rerun: wasm build green | 0 | wf_7b830d6a-b9a |
| shader | tui-shader =0.0.10 temet preset (WGSL intensity → Theme::temet() colors via StyleRule fn ptr) + example, feature-gated | sonnet | R2 | verified | src/shader.rs, examples/shader.rs | feature-matrix build + wgpu-absence + literal grep + orchestrator rerun | verify agent PASS (61 tests w/ feature, 0 wgpu default, no color literals); orchestrator rerun: default+shader builds green, wgpu count 0 | 0 | wf_7b830d6a-b9a |
| hot-reload | gallery --dump-svg/--watch/--watch-selftest via notify-debouncer-full | sonnet | R2 | verified | examples/gallery.rs only | independent selftest runs + code review + orchestrator rerun | verify agent PASS (real fs event, real re-raster, frame1≠frame2 assert); orchestrator rerun: WATCH SELFTEST PASS + SELFTEST PASS exit 0 | 0 | wf_7b830d6a-b9a |
| deprecation-fix | eliminate Picker::from_fontsize deprecation warnings preserving 8x16 pixel determinism | sonnet | R2 | blocked | src/cache.rs, src/widget.rs, examples/*, tests/* (constructor sites only) | — | died on session usage limit 2026-08-21 before completing; superseded by deprecation-fix2 | 1 | agent:deprecation-fix |
| deprecation-fix2 | same brief, fresh worker | sonnet | R2 | verified | same | orchestrator reran clippy/test/selftests | scoped #[allow(deprecated)] helpers per file (halfblocks() pins 10x20 — no non-deprecated 8x16 ctor exists); 13 sites; orchestrator rerun 2026-08-23: clippy 0 warnings, 58 tests green, SELFTEST PASS + WATCH SELFTEST PASS | 0 | agent:deprecation-fix2 |
| accept-adversary | adversarial acceptance check vs verbatim frozen contract | sonnet | R2 | verified | (read-only) | n/a | PASS all 6 criteria; one flag: criterion-2 wording "workspace member" vs delivered standalone crate (accepted deliberately — true membership would drag wasm deps into root builds); noted pre-existing teal/amber literals in console-shell thread's uncommitted tabs.rs/statusbar.rs (not this run's) | 0 | agent:accept-adversary |

Spend: R1=1 R2=12 R3=1 R4=1 takeovers=0 escalations=0

Dependency edges: upgrade → (web-demo, shader, hot-reload). shader/hot-reload Cargo.toml needs are pre-provisioned by `upgrade` so allowlists stay disjoint.

## Lessons & environment
- Toolchain: cargo/rustc 1.96.0; wasm32-unknown-unknown target NOT installed; no trunk/wasm-bindgen CLI. Criterion 2 needs only `cargo build --target wasm32-unknown-unknown`.
- Latest versions per crates.io (2026-08-21): ratatui 0.30.2, ratatui-image 11.0.6, rat-widget 3.2.1, tui-textarea 0.7.0.
- Repo committed at fcfdf51 but working tree carries uncommitted changes (console/statusbar/tabs units) — do not stash/discard. Baseline snapshot: scratchpad/baseline-status.txt + baseline-diff.patch (session 5fa0a04e).
- Recon (wf_aafc04a7-124, recovered from journal): ratatui 0.30 keeps root paths incl. `ratatui::crossterm` (default `crossterm` feature; verified on docs.rs 0.30.2); breaking: Backend assoc `Error` type + `clear_region`, block::Title removed (Into<Line>, TitlePosition), Style no longer impls Styled (Stylize methods on Style), Marker #[non_exhaustive], Flex::SpaceAround semantics changed (old = SpaceEvenly), layout-cache behind feature, MSRV 1.86. ratatui-image 11 requires ratatui ^0.30.1; API Rect→Size (Picker::new_protocol, area()→size(), StatefulProtocol::size_for, Halfblocks/Kitty/Sixel/ITerm2::new(Size)), FontSize now struct, Image::allow_clipping, Protocol::needs_placeholder; crossterm confined to Picker query path behind default `crossterm` feature — Halfblocks render path is pure Buffer cells (wasm-OK).
- ratzilla 0.3.1 (repo ratatui/ratzilla): requires ratatui ^0.30.1, `pub use ratatui`, DomBackend/CanvasBackend/WebGl2Backend + terminal.draw_web(); plain `cargo build --target wasm32-unknown-unknown` compiles; trunk only needed to bundle/serve (index.html needs `<link data-trunk rel="rust"/>`); crossterm not wasm-compatible → Picker autodetect unusable in browser, construct Halfblocks protocol directly.
- tui-shader 0.0.10 (Apr 2026): supports ratatui 0.30 via ratatui-core 0.1; pulls wgpu 27 (upstream at 30); API: ShaderCanvas StatefulWidget + ShaderCanvasState::new(WgslShader::Path|include_wgsl), CharacterRule/StyleRule map fragment output→cells; pre-1.0, pin =0.0.10; no documented headless/CI fallback (needs real GPU).
- notify 8.2.0: notify::recommended_watcher(tx) + RecursiveMode; debouncing companion notify-debouncer-full 0.7.0.
- LESSON (workflow): recon workers' StructuredOutput payloads ~5KB failed validation (embedded tool-call tags corrupted facts arrays) and workers then submitted placeholder junk ("test"/"a"). Mitigate: cap return payloads (~1.5KB), forbid placeholder submissions in prompt, recover real payloads from subagents/workflows/<run>/agent-*.jsonl failed attempts.
- Local API map (blast radius) captured in recon localApi result: Picker/StatefulImage/StatefulProtocol in cache.rs+widget.rs+examples+tests; ratatui::crossterm::event in console.rs/nav.rs/examples/tests; 60 #[test] fns total.

## Session log
- 2026-08-21 (unknown) @ fcfdf51 — thread created.
- 2026-08-21 (unknown) @ fcfdf51 — Intake: contract frozen for crate integrations (ratatui 0.30 upgrade, ratzilla web demo, tui-shader feature, notify hot reload)
- 2026-08-21 (unknown) @ fcfdf51 — Recon done (facts recovered from journal after StructuredOutput placeholder failure; lesson logged). Build workflow wf_7b830d6a-b9a dispatched: upgrade (R3) gates the fan-out of web-demo (R4 opus), shader (R2), hot-reload (R2), each with independent verify + 1 fix round.
- 2026-08-23 (unknown) @ fcfdf51 — DONE: all 6 criteria met, adversarial verifier PASS. ratatui 0.30.2 + ratatui-image 11 (58 tests, clippy 0 warnings), web-gallery/ ratzilla wasm demo builds, shader feature isolated (0 wgpu default), gallery --watch/--watch-selftest hot reload proven. Deprecation cleanup via scoped allows (no non-deprecated 8x16 ctor in ratatui-image 11). Uncommitted per commit-when-asked. Follow-ons: trunk serve of web demo, GPU-run of shader example, console-shell thread teal/amber literals.
