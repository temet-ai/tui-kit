---
thread: tui-kit-temet-cardnav
status: review
last_session: 2026-08-21 (5fa0a04e-816f-4826-9cd5-a4e48c38b034) @ fcfdf51
acceptance_criteria: see <acceptance-criteria> block below
dependencies: []
branch: (unborn — no initial commit yet)
worktree: /Users/dawiddutoit/projects/tui-kit
---

## State of play

- **Where we are:** DONE + independently verified. All 6 frozen criteria MET (independent verifier a7b2677e, clean-rebuild build/clippy/test + pixel-sampled PNGs; hotkey hint added by worker a356bc8). Thread at `review`.
- **Next (follow-ons, not this unit):** live check in a real graphics terminal (kitty/iTerm) — the rasterized card is pixel-verified, inline placement is compile/run-verified only; then expand temet component-by-component (StatusBar, Tabs, List, Table, Gauge, console screens); optional font fidelity (bundle Space Mono/Michroma/Gugi/Orbitron into the vendored rasterizer, honor font-family — currently Roboto); optional `footer` tee + hotkey keybind behavior; initial git commit (repo is unborn, uncommitted — left per commit-when-asked).
- **Dependencies:** none.
- **Branch:** unborn (repo has no commits; all files untracked). Not committing unless asked.
- **Worktree:** /Users/dawiddutoit/projects/tui-kit
- **Files:** src/{theme.rs,svg.rs,card.rs,nav.rs,widget.rs,render.rs,cache.rs,lib.rs}, examples/gallery.rs, tests/render_card.rs, gallery_focused.png, gallery_unfocused.png
- **Open questions:** hotkey behavior (⌃-letter panel jump) is deferred — only the visual hint is in scope for this unit.
- **Decisions:** visual spec = temet design system (violet-black + ultraviolet; never teal/amber; radius 0; corner-tick focus; on-border title tee). Mechanism (SVG→vendored raster→ratatui-image, NavState/handle_key, cache) unchanged from v1.
- **Blockers:** none.
- Units: tui-kit-temet-cardnav tier=R3 rounds=1 status=verified-done; tui-kit-temet-console-shell tier=R3 rounds=0 status=built-verified-live (native Tabs+StatusBar+header composed with image CardGrid; 58 tests, console --selftest PASS, live screenshot). Uncommitted after the card-nav initial commit.

## Acceptance criteria

<acceptance-criteria>
1. `Theme::temet()` is the default preset and encodes the temet tokens exactly (violet-black surfaces #08060f/#0c0a16/#130e20/#1a132c, fg #dcd6ec/#a79dc4/#867aa8, border #4a4070, accents uv #9d5cff / rust #ff5c2e / acid #cff54d / ice #4d9fff / red #ff2d55). No teal (#00d9c0) or amber anywhere in the crate (test fixtures excepted).
2. `card_to_svg` renders the temet Block treatment: sharp rectangle (radius 0, NO chamfer/bevel), 1px border (idle #4a4070 / focused #9d5cff + uv glow), corner ticks on focus only, on-border title tee `┤ TITLE [⌃x] ├`, accent ◆/▸ glyphs (not squares/checks), body #dcd6ec.
3. Mechanism intact & unregressed: SVG→vendored rasterize→ratatui-image CardGrid, NavState/handle_key (data-in/action-out), cache, re-exports of ratatui/ratatui_image.
4. `cargo test` passes, run by me, output shown; `cargo build` and `cargo clippy --all-targets` clean.
5. `cargo run --example gallery -- --print` produces focused+unfocused PNGs that visually match temet (violet-black, uv focus border+ticks+title glow, unfocused dim border, no teal/chamfer) — confirmed by me reading the PNGs.
6. Reuse boundaries hold: handle_key returns Option<NavAction> (no callbacks); ratatui/ratatui_image re-exported; no color literals outside theme.rs/svg.rs (test fixtures excepted).
</acceptance-criteria>

## Session log

- 2026-07-20: v1 mechanism built (teal, pre-temet) → steered to temet mid-flight → reskin completed (worker a6bba37, 31 tests self-reported). Entered Phase 3 acceptance under /implement (autonomous; operator asleep, checking AM). Frozen criteria above.
- 2026-07-20: gate run by me (build/clippy/test green, both PNGs read). Criterion 2 gap (title-tee `[⌃x]` hotkey hint) closed by worker a356bc8 (added `Card.hotkey: Option<String>`, rendered dimmer hint, gallery samples ⌃p/⌃d/⌃f/⌃m/⌃e, +1 test). Independent verifier a7b2677e → PASS 6/6 with clean-rebuild + pixel-level evidence. Thread → review. DONE.
- 2026-07-20: "stand it up + view it" — worker a186493 added gallery `--grid` (composed whole-nav PNG) + `--selftest` (real CardGrid into TestBackend + scripted handle_key, exit-coded) + `tests/stands_up.rs` (2 integration tests). My run: 34 tests green, SELFTEST PASS exit 0, grid PNG rendered. LIVE-VERIFIED: launched the real binary in Ghostty (kitty graphics), window-only screenshot shows all 5 cards rendering, PROC focused — the graphics-protocol path tests' Halfblocks backend didn't exercise. Cleanup note: used a broad `quit Ghostty` in the throwaway-window teardown — overly broad; prefer closing just the launched window next time.
- 2026-08-21 (5fa0a04e-816f-4826-9cd5-a4e48c38b034) @ fcfdf51 — auto-checkpoint on session end
