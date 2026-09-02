---
title: TUI Design — Panels proposal
status: proposal
created: 2026-07-24
updated: 2026-07-24
summary: Editable decision worksheet for panel-internal geometry foundations. Not normative TUI Design.
---

# TUI Design — Panels proposal

**Status: proposal, not normative.** This worksheet explores panel-internal geometry only.
It does not change [`STYLE_GUIDE.md`](STYLE_GUIDE.md), does not assign final panel rule IDs,
and does not make the current Rust widgets conformant. Once decisions are made, accepted
values can be promoted into the normative guide and implementation work can follow.

The designed rendition is [`PANELS.html`](PANELS.html). Regenerate it with
[`build_panels_doc.py`](build_panels_doc.py). The builder keeps every editable number in
one `PROPOSAL_CONSTANTS` section and asserts the exact width of every terminal row.

## Proposal constants

These are candidates to discuss, not settled values. Every proposed number is labelled
`PROPOSAL` and has a rationale.

| Candidate | PROPOSAL value | Rationale |
|---|---:|---|
| Horizontal panel padding | **PROPOSAL: 1 cell per side** | Preserves a visible inset without spending scarce width. |
| Minimum related-item gap | **PROPOSAL: 1 cell** | Absolute compact-mode floor; adjacent fields still remain separable. |
| Preferred related-item gap | **PROPOSAL: 2 cells** | Stronger grouping at normal widths without creating table “rivers.” |
| Maximum related-item gap | **PROPOSAL: 4 cells** | Prevents unused width from pushing related values to opposite edges. |
| State cell width | **PROPOSAL: 7 cells** | Fits the candidate vocabulary `live`, `busy`, `idle`, and `error` with stable alignment. |
| Label minimum / preferred / maximum | **PROPOSAL: 6 / 12 / 18 cells** | Keeps identity readable, but returns width to contextual resource values first. |
| Value minimum | **PROPOSAL: 10 cells** | Fits compact contextual values such as `60% · 1.2/2G`. |
| Action minimum | **PROPOSAL: 7 cells** | Fits `inspect`; the keybinding remains in navigation help or the detail view. |
| Table content maximum | **PROPOSAL: 68 cells** | Leaves extra widescreen width trailing after the related fields rather than between them. |
| Compact container threshold | **PROPOSAL: under 44 inner cells** | Switches resource values to compact-but-contextual forms before columns collide. |
| Wide container threshold | **PROPOSAL: at least 64 inner cells** | Restores full unit precision and the context/action cell when it fits naturally. |
| Banding threshold | **PROPOSAL: 8 data rows** | Short lists stay quiet; scanning help appears when a list becomes visually repetitive. |
| Banding cadence | **PROPOSAL: every 2nd data row** | The smallest stable alternating pattern. |
| Scroll gutter | **PROPOSAL: 1 cell** | Reserves a stable edge for scroll position without changing cell ownership. |
| Width validation set | **PROPOSAL: 60, 80, 120, and 160 columns** | Exercises stacked narrow, floor, roomy, and widescreen compositions. |
| Specimen heights | **PROPOSAL: 22, 24, 28, and 32 rows respectively** | Makes the complete-screen examples legible while exercising vertical scroll behavior. |

## Row anatomy and exact cell ownership

Candidate ownership is left-to-right and stable:

```text
│ state   ·· label       ·· value/context       ·· action  │ trailing unused width │
  └ 7 ──┘    └ 6–18 ──┘    └ flex, min 10 ──┘    └ min 7 ┘
```

- `state` owns the canonical state word and its semantic color. It never owns a dot,
  diamond, pill, or duplicate word.
- `label` owns the row identity (`api`, `worker`, `cache`).
- `value/context` owns a meaningful measurement with its denominator, unit, workload, or
  other interpretation.
- `action` owns the contextual action only when the whole cell fits. At compact widths the
  same action remains available through selection plus the detail/preview panel; it is not
  clipped into an ambiguous fragment.
- Unused width is trailing row space. It is not redistributed into elastic gaps.

The separators in the diagram show the **PROPOSAL: 2-cell preferred gap**. The literal
middle dot is explanatory annotation, not an indicator glyph.

## Candidate gaps between related inline items

```text
PROPOSAL minimum (1):    live api
PROPOSAL preferred (2):  live  api
PROPOSAL maximum (4):    live    api
Wrong:                   live                              api
```

The gap adapts only inside the **PROPOSAL: 1–4-cell range**. Once the maximum is reached,
additional width stays after the row content. This preserves the visual relationship
between state, identity, value, and action.

## Table alignment and unused width

Numbers align by meaning, not by the terminal edge:

```text
live   api       1.20 / 2.00 GiB · 60%  inspect
busy   worker    6 / 8 jobs · queue 2    inspect
idle   cache     0 B / 512 MiB · unused  inspect
                                              trailing space ────────────
```

The value column is left-aligned because its contents mix ratios, units, and words. If a
future homogeneous numeric column is introduced, the numbers may right-align inside that
column; that does not justify right-aligning the entire value/action group to the panel
edge. The **PROPOSAL: 68-cell table content maximum** keeps related values together on wide
screens.

## Alternating row banding

```text
live   api       1.20 / 2.00 GiB · 60%
░busy░░worker░░░░6 / 8 jobs · queue 2░░░░░
idle   cache     0 B / 512 MiB · unused
░live░░gateway░░░14 / 20 conn · 70%░░░░░░░░
```

The `░` characters visualize background treatment in this Markdown worksheet; the rendered
artifact uses background color and does not insert glyphs. Banding starts only when the
dataset reaches the **PROPOSAL: 8-row threshold**, then shades every **PROPOSAL: 2nd row**.
Selection replaces the band background across the whole row.

## Adaptive resource/value representation

The meaning is constant while precision adapts to the panel's inner width:

```text
wide:     1.20 / 2.00 GiB · 60%
standard: 1.2/2.0 GiB · 60%
compact:  60% · 1.2/2G
```

All three representations preserve used, capacity, and percentage. Equivalent contextual
examples:

```text
wide:     6 / 8 jobs · queue 2
compact:  6/8 · queue 2

wide:     0 B / 512 MiB · unused
compact:  unused · 0/512M
```

Wrong examples are `3 entries` without what the entries represent, and standalone `0%`
without a capacity, quantity, or state. Compact means less precision, never less meaning.
The rendered worksheet shows the same service table in complete TUI Design screens at the
**PROPOSAL validation widths: 60, 80, 120, and 160 columns**, including the app path and
pinned navigation-only final help row.

## Overflow, wrapping, and scrolling

Content is never silently shortened:

```text
request target
  /organizations/acme-production/regions/eu-west-1/
  services/semantic-index/replicas/primary
  ↑ wrapped continuation remains selectable with its owning row

services                                              4–9 / 12
  ↓ additional rows remain reachable with j/k and PgDn
```

- Wrap inspection values onto continuation rows before dropping content.
- A continuation aligns with the start of the owning value cell, derived from panel padding,
  state width, and the active gap. It does not introduce a new magic indent.
- Scroll vertically inside the panel. Show position or remaining-row context; do not pretend
  the visible slice is the whole dataset.
- Paginate only when the data source already has meaningful pages.
- Horizontal scrolling is acceptable for structure where wrapping would change meaning,
  but the viewport must expose the current horizontal position.
- Do not use ellipsis, manual shortened previews, or “show more” as a substitute for
  preserving inspectable content.

## Semantic consistency

The same state uses the same word, cell, and semantic color everywhere:

```text
Right
live   api       1.2/2.0 GiB · 60%
live   gateway   14/20 conn · 70%

Wrong
● UP       api
◇ ACTIVE   gateway
```

Candidate vocabulary in this worksheet is `live`, `busy`, `idle`, and `error`. This
vocabulary itself still needs a decision, but mixing `ACTIVE`, `UP`, and `live` for one
state is not a candidate. State is never duplicated through pills, diamonds, indicator
dots, or a second state word in another cell.

## Decision checklist

- [ ] Accept or change **PROPOSAL horizontal padding: 1 cell per side**.
- [ ] Accept or change the **PROPOSAL related gap range: 1–4 cells** and
      **PROPOSAL preferred gap: 2 cells**.
- [ ] Accept or change **PROPOSAL state width: 7 cells** and the candidate state vocabulary.
- [ ] Accept or change **PROPOSAL label range: 6–18 cells** and
      **PROPOSAL preferred label width: 12 cells**.
- [ ] Accept or change **PROPOSAL value minimum: 10 cells** and
      **PROPOSAL action minimum: 7 cells**.
- [ ] Accept or change **PROPOSAL table content maximum: 68 cells**.
- [ ] Accept or change **PROPOSAL compact/wide thresholds: under 44 / at least 64 inner
      cells**.
- [ ] Accept or change **PROPOSAL banding threshold/cadence: 8 rows / every 2nd row**.
- [ ] Accept or change **PROPOSAL scroll gutter: 1 cell** and the scroll-position wording.
- [ ] Confirm compact resource forms preserve the required context for each data domain.
- [ ] Confirm actions move to selection/detail context when their cell cannot fit.
- [ ] Confirm wrapping, scrolling, and pagination cover all overflow cases without silent
      truncation.
- [ ] Only after the decisions above: assign final panel rule IDs and promote accepted rules
      into [`STYLE_GUIDE.md`](STYLE_GUIDE.md).
