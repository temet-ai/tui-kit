#!/usr/bin/env python3
"""Generate PANELS.html, the editable panel-geometry proposal worksheet.

All numeric design candidates live in PROPOSAL_CONSTANTS. Every terminal row is
asserted against its declared cell width before HTML is written.
"""

import base64
import html
import textwrap
from pathlib import Path


# ---------------------------------------------------------------------------
# PROPOSAL CONSTANTS — edit candidates here, then regenerate and inspect.
# None of these values is normative until promoted into STYLE_GUIDE.md.
# ---------------------------------------------------------------------------

PROPOSAL_CONSTANTS = {
    "panel_padding_x": 1,
    "inline_gap_min": 1,
    "inline_gap_preferred": 2,
    "inline_gap_max": 4,
    "state_width": 7,
    "label_width_min": 6,
    "label_width_preferred": 12,
    "label_width_max": 18,
    "value_width_min": 10,
    "action_width_min": 7,
    "table_content_max": 68,
    "compact_inner_below": 44,
    "wide_inner_at_least": 64,
    "banding_min_rows": 8,
    "banding_every": 2,
    "scroll_gutter": 1,
    "screen_sizes": ((60, 22), (80, 24), (120, 28), (160, 32)),
}


HERE = Path(__file__).resolve().parent
OUT = HERE / "PANELS.html"
FONT = base64.b64encode((HERE / "assets" / "term-mono.woff2").read_bytes()).decode()
ASSERTED_LINES = 0


def esc(value):
    return html.escape(value, quote=False)


def visible(segments):
    return "".join(text for _, text in segments)


def render_line(segments, width, context):
    """Render segments and prove their terminal-cell width."""
    global ASSERTED_LINES
    raw = visible(segments)
    assert len(raw) <= width, (
        f"{context}: overflow by {len(raw) - width} cells "
        f"(expected {width}, got {len(raw)}): {raw!r}"
    )
    ASSERTED_LINES += 1
    rendered = "".join(
        f'<span class="{cls}">{esc(text)}</span>' if cls else esc(text)
        for cls, text in segments
    )
    return rendered + (" " * (width - len(raw)))


def fit(value, width):
    """Pad a value known to fit. Proposal specimens never truncate values."""
    assert len(value) <= width, f"value does not fit owned cell {width}: {value!r}"
    return value + (" " * (width - len(value)))


def title_line(border_class, title, width):
    shortcut = title[0]
    rest = title[1:]
    tail = width - 3 - len(title) - 2
    assert tail >= 0, f"panel title {title!r} does not fit width {width}"
    return (
        f'<span class="{border_class}">┌─ </span>'
        f'<span class="shortcut">{esc(shortcut)}</span>'
        f'<span class="title">{esc(rest)}</span>'
        f'<span class="{border_class}"> {"─" * tail}┐</span>'
    )


def panel(border_class, title, width, rows, height, context):
    """Build a complete bordered panel.

    A row may be a segment list or (row_css_class, segment list). The segment
    list owns exactly the panel's inner width after render_line padding.
    """
    inner = width - 2
    body = list(rows)
    assert len(body) <= height - 2, (
        f"{context}: {len(body)} body rows do not fit panel height {height}"
    )
    while len(body) < height - 2:
        body.append([])
    output = [title_line(border_class, title, width)]
    for index, row in enumerate(body):
        row_class = None
        if isinstance(row, tuple):
            row_class, row = row
        rendered = render_line(row, inner, f"{context} body row {index + 1}")
        if row_class:
            rendered = f'<span class="{row_class}">{rendered}</span>'
        output.append(
            f'<span class="{border_class}">│</span>{rendered}'
            f'<span class="{border_class}">│</span>'
        )
    output.append(f'<span class="{border_class}">└{"─" * (width - 2)}┘</span>')
    assert len(output) == height
    return output


SERVICES = (
    ("live", "api", "memory", "inspect"),
    ("busy", "worker", "jobs", "inspect"),
    ("idle", "cache", "cache", "inspect"),
    ("live", "gateway", "connections", "inspect"),
    ("busy", "indexer", "documents", "inspect"),
    ("live", "events", "rate", "inspect"),
    ("idle", "backup", "backup", "inspect"),
    ("live", "search", "shards", "inspect"),
    ("busy", "queue", "queue", "inspect"),
)

VALUE_FORMS = {
    "wide": {
        "memory": "1.20/2.00 GiB · 60%",
        "jobs": "6/8 jobs · queue 2",
        "cache": "0 B/512 MiB · unused",
        "connections": "14/20 conn · 70%",
        "documents": "842/1.2k docs · 70%",
        "rate": "320/500 msg/s · 64%",
        "backup": "0 B/8 GiB · waiting",
        "shards": "5/6 shards · 83%",
        "queue": "18/40 jobs · 45%",
    },
    "standard": {
        "memory": "1.2/2.0 GiB · 60%",
        "jobs": "6/8 jobs · queue 2",
        "cache": "0/512 MiB · unused",
        "connections": "14/20 conn · 70%",
        "documents": "842/1.2k · 70%",
        "rate": "320/500 msg/s",
        "backup": "waiting · 0/8G",
        "shards": "5/6 shards · 83%",
        "queue": "18/40 jobs · 45%",
    },
    "compact": {
        "memory": "60%·1.2/2G",
        "jobs": "6/8·queue2",
        "cache": "unused·0/512",
        "connections": "70%·14/20",
        "documents": "70%·842/1k",
        "rate": "320/500msg/s",
        "backup": "waiting·0/8G",
        "shards": "83%·5/6",
        "queue": "45%·18/40",
    },
}


def column_plan(inner):
    """Return a container-based column plan using only proposal constants."""
    c = PROPOSAL_CONSTANTS
    padding = c["panel_padding_x"]
    gutter = c["scroll_gutter"]
    budget = inner - (padding * 2) - gutter
    content_budget = min(budget, c["table_content_max"])
    if inner >= c["wide_inner_at_least"]:
        mode = "wide"
        gap = c["inline_gap_preferred"]
        label = c["label_width_preferred"]
        action = c["action_width_min"]
        fixed = c["state_width"] + label + action + (gap * 3)
        value = max(c["value_width_min"], min(24, content_budget - fixed))
    elif inner >= c["compact_inner_below"]:
        mode = "standard"
        gap = c["inline_gap_preferred"]
        label = 10
        action = c["action_width_min"]
        fixed = c["state_width"] + label + action + (gap * 3)
        value = max(c["value_width_min"], content_budget - fixed)
    else:
        mode = "compact"
        gap = c["inline_gap_min"]
        action = 0
        label_room = content_budget - c["state_width"] - (gap * 2) - c["value_width_min"]
        label = max(c["label_width_min"], min(8, label_room))
        value = content_budget - c["state_width"] - label - (gap * 2)
    assert value >= c["value_width_min"]
    used = (
        c["state_width"]
        + gap
        + label
        + gap
        + value
        + ((gap + action) if action else 0)
    )
    assert used <= content_budget
    return {
        "mode": mode,
        "padding": padding,
        "gutter": gutter,
        "budget": budget,
        "content_budget": content_budget,
        "gap": gap,
        "state": c["state_width"],
        "label": label,
        "value": value,
        "action": action,
        "used": used,
    }


def service_segments(inner, state, label, value_kind, action, header=False):
    plan = column_plan(inner)
    gap = " " * plan["gap"]
    if header:
        value = "resource"
    else:
        candidates = (
            [VALUE_FORMS["wide"][value_kind], VALUE_FORMS["standard"][value_kind]]
            if plan["mode"] == "wide"
            else [VALUE_FORMS["standard"][value_kind]]
            if plan["mode"] == "standard"
            else []
        )
        candidates.append(VALUE_FORMS["compact"][value_kind])
        value = next(candidate for candidate in candidates if len(candidate) <= plan["value"])
    state_text = "state" if header else state
    label_text = "service" if header else label
    action_text = "action" if header else action
    classes = {
        "live": "success",
        "busy": "warning",
        "idle": "muted",
        "error": "error",
    }
    segments = [
        (None, " " * plan["padding"]),
        ("muted" if header else classes[state], fit(state_text, plan["state"])),
        (None, gap),
        ("muted" if header else "primary", fit(label_text, plan["label"])),
        (None, gap),
        ("muted" if header else "number", fit(value, plan["value"])),
    ]
    if plan["action"]:
        segments.extend(
            [
                (None, gap),
                ("muted" if header else "action", fit(action_text, plan["action"])),
            ]
        )
    trailing = plan["budget"] - plan["used"]
    segments.extend(
        [
            (None, " " * trailing),
            ("scroll", " " if header else "░"),
            (None, " " * plan["padding"]),
        ]
    )
    assert len(visible(segments)) == inner
    return segments


def service_rows(inner, body_capacity):
    """Build a visible service slice, keeping scroll context when rows remain."""
    assert body_capacity >= 3
    rows = [service_segments(inner, "", "", "", "", header=True)]
    visible_capacity = body_capacity - 2
    shown = SERVICES[:visible_capacity]
    should_band = len(SERVICES) >= PROPOSAL_CONSTANTS["banding_min_rows"]
    for index, service in enumerate(shown):
        row = service_segments(inner, *service)
        band = should_band and ((index + 1) % PROPOSAL_CONSTANTS["banding_every"] == 0)
        rows.append(("band", row) if band else row)
    remaining = len(SERVICES) - len(shown)
    if remaining:
        message = f" {len(shown) + 1}–{len(SERVICES)} below · PgDn"
    else:
        message = f" {len(SERVICES)} services · top"
    rows.append([("muted", message)])
    return rows


def nav_rows():
    return [
        ("selected", [(None, " services")]),
        [(None, " resources")],
        [(None, " events")],
        [(None, " settings")],
    ]


def wrap_full_text(prefix, value, inner):
    """Wrap complete text without elision; continuation stays visibly owned."""
    width = inner - 2
    lines = [f" {prefix}"]
    if value.startswith("/"):
        wrapped = []
        current = ""
        for part in [f"{part}/" for part in value.strip("/").split("/")]:
            candidate = current + part
            if current and len(candidate) > width - 2:
                wrapped.append(current)
                current = part
            else:
                current = candidate
        if current:
            wrapped.append(current[:-1])
    else:
        wrapped = textwrap.wrap(
            value,
            width=width - 2,
            break_long_words=True,
            break_on_hyphens=False,
            replace_whitespace=False,
            drop_whitespace=False,
        )
    assert all(len(part) <= width - 2 for part in wrapped)
    lines.extend(f"  {part}" for part in wrapped)
    return [[("muted" if index == 0 else "primary", line)] for index, line in enumerate(lines)]


def detail_rows(inner, capacity):
    if capacity <= 8:
        rows = [
            [("muted", " selected")],
            [("success", " live"), (None, "  api")],
            [("muted", " resource")],
            [("number", " 1.20/2.00 GiB · 60%")],
            [("muted", " action")],
            [("action", " inspect")],
            [("muted", " full value in this view")],
        ]
        return rows[:capacity]
    rows = [
        [("muted", " selected row")],
        [("success", " live"), (None, "  api")],
        [("muted", " resource  "), ("number", "1.20/2.00 GiB · 60%")],
        [("muted", " action    "), ("action", "inspect")],
        [],
    ]
    rows.extend(
        wrap_full_text(
            "request target",
            "/organizations/acme-production/regions/eu-west-1/services/"
            "semantic-index/replicas/primary",
            inner,
        )
    )
    rows.append([])
    rows.append([("muted", " wraps fully · j/k scroll")])
    return rows[:capacity]


def screen(width, height):
    """Render the universal layout at one proposal viewport."""
    assert (width, height) in PROPOSAL_CONSTANTS["screen_sizes"]
    content_height = height - 2
    if width == 60:
        nav_width = 12
        right_width = width - nav_width
        top_height = content_height // 2
        bottom_height = content_height - top_height
        nav = panel("border", "nav", nav_width, nav_rows(), content_height, f"{width} nav")
        services = panel(
            "focus",
            "services",
            right_width,
            service_rows(right_width - 2, top_height - 2),
            top_height,
            f"{width} services",
        )
        detail = panel(
            "border",
            "api",
            right_width,
            detail_rows(right_width - 2, bottom_height - 2),
            bottom_height,
            f"{width} detail",
        )
        right = services + detail
        content = [left + right_row for left, right_row in zip(nav, right)]
    else:
        nav_width = 12
        remaining = width - nav_width
        middle_width = remaining // 2
        detail_width = remaining - middle_width
        nav = panel("border", "nav", nav_width, nav_rows(), content_height, f"{width} nav")
        services = panel(
            "focus",
            "services",
            middle_width,
            service_rows(middle_width - 2, content_height - 2),
            content_height,
            f"{width} services",
        )
        detail = panel(
            "border",
            "api",
            detail_width,
            detail_rows(detail_width - 2, content_height - 2),
            content_height,
            f"{width} detail",
        )
        content = [
            left + middle + right
            for left, middle, right in zip(nav, services, detail)
        ]
    app = render_line(
        [
            (None, " "),
            ("emphasis", "TUI Design"),
            ("muted", " / "),
            (None, "panels"),
            ("muted", " / "),
            ("emphasis", "services"),
        ],
        width,
        f"{width} app path",
    )
    if width == 60:
        help_segments = [
            (None, " "),
            ("key", "j/k"),
            ("muted", " move · "),
            ("key", "tab"),
            ("muted", " panel · "),
            ("key", "enter"),
            ("muted", " open · "),
            ("key", "?"),
            ("muted", " help · "),
            ("key", "q"),
            ("muted", " quit"),
        ]
    else:
        help_segments = [
            (None, " "),
            ("key", "j/k"),
            ("muted", " move · "),
            ("key", "tab"),
            ("muted", " panel · "),
            ("key", "enter"),
            ("muted", " inspect · "),
            ("key", "pgdn"),
            ("muted", " scroll · "),
            ("key", "?"),
            ("muted", " help · "),
            ("key", "q"),
            ("muted", " quit"),
        ]
    help_row = render_line(help_segments, width, f"{width} help")
    rows = [f'<span class="appbar">{app}</span>', *content, f'<span class="helpbar">{help_row}</span>']
    assert len(rows) == height
    return "\n".join(rows)


SCREENS = {
    width: screen(width, height)
    for width, height in PROPOSAL_CONSTANTS["screen_sizes"]
}


def constant_rows():
    labels = {
        "panel_padding_x": "horizontal panel padding",
        "inline_gap_min": "minimum inline gap",
        "inline_gap_preferred": "preferred inline gap",
        "inline_gap_max": "maximum inline gap",
        "state_width": "state cell width",
        "label_width_min": "label minimum",
        "label_width_preferred": "label preferred",
        "label_width_max": "label maximum",
        "value_width_min": "value minimum",
        "action_width_min": "action minimum",
        "table_content_max": "table content maximum",
        "compact_inner_below": "compact threshold",
        "wide_inner_at_least": "wide threshold",
        "banding_min_rows": "banding threshold",
        "banding_every": "banding cadence",
        "scroll_gutter": "scroll gutter",
    }
    units = {
        "banding_min_rows": "data rows",
        "banding_every": "rows",
    }
    rows = []
    for key, label in labels.items():
        value = PROPOSAL_CONSTANTS[key]
        unit = units.get(key, "cell" if value == 1 else "cells")
        rows.append(
            f"<tr><td>{esc(label)}</td><td><strong>PROPOSAL: {value} {unit}</strong></td>"
            f"<td><code>{esc(key)}</code></td></tr>"
        )
    return "".join(rows)


FONT_RULES = f"""
@font-face{{font-family:'TermMono';src:url(data:font/woff2;base64,{FONT}) format('woff2');font-display:block}}
html{{-webkit-text-size-adjust:100%;text-size-adjust:100%}}
"""

CSS = FONT_RULES + """
:root{
  --bg-base:#0a0812;--bg-surface:#12101a;--bg-overlay:#1a162a;--bg-selection:#44475a;
  --text-primary:#f8f8f2;--text-secondary:#e9d5ff;--text-muted:#6272a4;--text-emphasis:#ffffff;
  --accent-primary:#e135ff;--accent-secondary:#80ffea;--success:#50fa7b;
  --warning:#f1fa8c;--error:#ff6363;--number:#ffb86c;--line:#44475a;
}
*{box-sizing:border-box}
body{margin:0;background:var(--bg-base);color:var(--text-primary);
  font-family:"JetBrains Mono","Fira Code",ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;
  font-size:14px;line-height:1.55}
.wrap{max-width:1240px;margin:0 auto;padding:42px 24px 96px}
h1,h2,h3{color:var(--text-emphasis);text-wrap:balance}
h1{font-size:30px;margin:0 0 8px}
h2{font-size:19px;margin:44px 0 8px;padding-left:12px;border-left:3px solid var(--accent-primary)}
h3{font-size:14px;color:var(--accent-secondary);margin:22px 0 8px}
p{max-width:92ch}
.lede{color:var(--text-secondary)}
.proposal{display:inline-block;border:1px solid var(--accent-primary);color:var(--accent-primary);
  padding:1px 7px;font-size:11px;font-weight:700}
a{color:var(--accent-secondary)}
code{color:var(--accent-secondary)}
.notice,.card{background:var(--bg-surface);border:1px solid var(--line);padding:16px 18px}
.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(280px,1fr));gap:14px}
table{width:100%;border-collapse:collapse;font-size:12px}
th,td{text-align:left;padding:7px 9px;border-bottom:1px solid rgba(98,114,164,.24);vertical-align:top}
th{color:var(--text-muted);font-weight:400}
pre{font-family:'TermMono',"JetBrains Mono",ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.term{background:var(--bg-base);border:1px solid var(--line);overflow-x:auto;margin:14px 0 4px}
.term pre{margin:0;padding:12px 14px;line-height:1.0;color:var(--text-primary)}
.w60 pre{font-size:min(16px,calc((min(100vw,1240px) - 82px)/36.2))}
.w80 pre{font-size:min(16px,calc((min(100vw,1240px) - 82px)/48.2))}
.w120 pre{font-size:min(13px,calc((min(100vw,1240px) - 82px)/72.3))}
.w160 pre{font-size:min(11px,calc((min(100vw,1240px) - 82px)/96.4))}
.caption{color:var(--text-muted);font-size:11.5px;margin:5px 0 20px}
.border{color:var(--text-muted)}.focus{color:var(--accent-primary)}
.shortcut{color:var(--accent-secondary);font-weight:700;text-decoration:underline}
.title,.emphasis{color:var(--text-emphasis);font-weight:700}
.primary{color:var(--text-primary)}.muted{color:var(--text-muted)}
.success{color:var(--success)}.warning{color:var(--warning)}.error{color:var(--error)}
.number{color:var(--number)}.action,.key{color:var(--accent-secondary);font-weight:700}
.selected{background:var(--bg-selection);color:var(--text-emphasis)}
.band{background:rgba(98,114,164,.10)}
.scroll{color:var(--accent-muted,#9580ff)}
.appbar,.helpbar{background:var(--bg-surface)}
.anatomy{display:grid;grid-template-columns:7fr 12fr 24fr 7fr 10fr;border:1px solid var(--line)}
.anatomy div{padding:9px;border-right:1px solid var(--line);min-width:0}
.anatomy div:last-child{border-right:0;color:var(--text-muted)}
.anatomy b{display:block;color:var(--accent-secondary);font-size:11px}
.sample{background:var(--bg-base);border:1px solid var(--line);padding:12px 14px;overflow-x:auto}
.band-demo div{padding:4px 8px}.band-demo div:nth-child(even){background:rgba(98,114,164,.10)}
.rightwrong{display:grid;grid-template-columns:1fr 1fr;gap:14px}
.wrong{border-color:var(--error)}.right{border-color:var(--success)}
.wrong h3{color:var(--error)}.right h3{color:var(--success)}
footer{margin-top:56px;padding-top:16px;border-top:1px solid var(--line);color:var(--text-muted);font-size:11px}
@media(max-width:700px){.rightwrong{grid-template-columns:1fr}.anatomy{grid-template-columns:1fr}}
"""


CONTENT = f"""<title>TUI Design — Panels proposal</title>
<style>{CSS}</style>
<div class="wrap">
<header>
  <span class="proposal">PROPOSAL · NOT NORMATIVE</span>
  <h1>TUI Design — Panels</h1>
  <p class="lede">An editable decision worksheet for panel-internal geometry foundations.
  It does not assign final panel rule IDs and does not change
  <a href="STYLE_GUIDE.md">STYLE_GUIDE.md</a>. All numeric candidates live in the
  <code>PROPOSAL_CONSTANTS</code> section of
  <a href="build_panels_doc.py">build_panels_doc.py</a>.</p>
</header>

<section>
  <h2>Editable constants</h2>
  <div class="card"><table>
    <tr><th>candidate</th><th>value</th><th>builder key</th></tr>
    {constant_rows()}
  </table></div>
</section>

<section>
  <h2>Row anatomy and cell ownership</h2>
  <p class="lede">Ownership stays left-to-right. The action cell exists only when it fits
  whole; unused width stays trailing instead of stretching relationships.</p>
  <div class="anatomy">
    <div><b>state</b>PROPOSAL: 7 cells</div>
    <div><b>label</b>PROPOSAL: 6–18 cells</div>
    <div><b>value / context</b>flex · PROPOSAL min 10</div>
    <div><b>action</b>PROPOSAL min 7</div>
    <div><b>unused</b>trailing space</div>
  </div>
  <div class="sample"><pre><span class="success">live   </span>  api           <span class="number">1.20/2.00 GiB · 60%</span>  <span class="action">inspect</span></pre></div>
</section>

<section>
  <h2>Related-item gaps</h2>
  <div class="grid">
    <div class="card"><h3>PROPOSAL minimum: 1 cell</h3><pre>live api</pre></div>
    <div class="card"><h3>PROPOSAL preferred: 2 cells</h3><pre>live  api</pre></div>
    <div class="card"><h3>PROPOSAL maximum: 4 cells</h3><pre>live    api</pre></div>
  </div>
  <p class="lede">After the maximum, new width remains after the row content. It never
  becomes an elastic chasm between related cells.</p>
</section>

<section>
  <h2>Alignment and unused width</h2>
  <div class="sample"><pre><span class="success">live   </span>  api           <span class="number">1.20/2.00 GiB · 60%</span>  <span class="action">inspect</span>
<span class="warning">busy   </span>  worker        <span class="number">6/8 jobs · queue 2</span>    <span class="action">inspect</span>
<span class="muted">idle   </span>  cache         <span class="number">0 B/512 MiB · unused</span>  <span class="action">inspect</span>
                                                      <span class="muted">trailing unused width</span></pre></div>
</section>

<section>
  <h2>Alternating row banding</h2>
  <p class="lede">PROPOSAL: begin at 8 data rows and shade every 2nd row. The treatment is
  background only; it inserts no glyph. Selection replaces the band across the whole row.</p>
  <div class="card band-demo">
    <div><span class="success">live</span> &nbsp; api &nbsp; <span class="number">1.20/2.00 GiB · 60%</span></div>
    <div><span class="warning">busy</span> &nbsp; worker &nbsp; <span class="number">6/8 jobs · queue 2</span></div>
    <div><span class="muted">idle</span> &nbsp; cache &nbsp; <span class="number">0 B/512 MiB · unused</span></div>
    <div><span class="success">live</span> &nbsp; gateway &nbsp; <span class="number">14/20 conn · 70%</span></div>
  </div>
</section>

<section>
  <h2>Adaptive resource values</h2>
  <div class="grid">
    <div class="card"><h3>wide</h3><pre>1.20/2.00 GiB · 60%</pre></div>
    <div class="card"><h3>standard</h3><pre>1.2/2.0 GiB · 60%</pre></div>
    <div class="card"><h3>compact</h3><pre>60% · 1.2/2G</pre></div>
  </div>
  <p class="lede">Precision adapts; meaning does not. Standalone <code>3 entries</code> and
  <code>0%</code> are not contextual resource representations.</p>
</section>

<section>
  <h2>Same specimen across complete screens</h2>
  <p class="lede">Every screen keeps the app path, universal panel frame, and pinned final
  help row containing navigation only. At narrow width the right panels stack as required
  by <a href="LAYOUT.md">LAYOUT.md</a>.</p>
  <div class="term w60"><pre>{SCREENS[60]}</pre></div>
  <p class="caption"><strong>PROPOSAL specimen: 60×22.</strong> Stacked right panels;
  adaptive resource forms remain contextual.</p>
  <div class="term w80"><pre>{SCREENS[80]}</pre></div>
  <p class="caption"><strong>PROPOSAL specimen: 80×24.</strong> The floor composition creates
  the tightest individual service panel, so compact forms activate.</p>
  <div class="term w120"><pre>{SCREENS[120]}</pre></div>
  <p class="caption"><strong>PROPOSAL specimen: 120×28.</strong> Standard columns include the
  contextual action without pushing it to the far edge.</p>
  <div class="term w160"><pre>{SCREENS[160]}</pre></div>
  <p class="caption"><strong>PROPOSAL specimen: 160×32.</strong> Full precision returns;
  unused width remains trailing.</p>
</section>

<section>
  <h2>Overflow, wrapping, and scrolling</h2>
  <div class="sample"><pre><span class="muted">request target</span>
  /organizations/acme-production/regions/eu-west-1/
  services/semantic-index/replicas/primary
<span class="muted">services                                      4–9 / 12 · PgDn</span></pre></div>
  <p class="lede">The full target wraps. The table reports its visible slice and remains
  scrollable. Structure that cannot wrap may scroll horizontally with visible position.
  Inspection content is never silently shortened.</p>
</section>

<section>
  <h2>Semantic consistency</h2>
  <div class="rightwrong">
    <div class="card right"><h3>Right</h3><pre><span class="success">live</span>  api
<span class="success">live</span>  gateway</pre><p>Same word, cell, and semantic color.</p></div>
    <div class="card wrong"><h3>Wrong</h3><pre>● UP      api
◇ ACTIVE  gateway</pre><p>Indicator shapes and duplicate vocabulary obscure one state.</p></div>
  </div>
</section>

<section>
  <h2>Decision pass</h2>
  <div class="notice">
    <p>Work through padding, gap range, cell widths, content maximum, container thresholds,
    banding, scroll context, adaptive value forms, and the candidate state vocabulary in
    <a href="PANELS.md">PANELS.md</a>. Only accepted outcomes move into the normative guide.</p>
  </div>
</section>

<footer>
  Proposal source: <a href="PANELS.md">PANELS.md</a> · normative layout:
  <a href="LAYOUT.md">LAYOUT.md</a> · normative system:
  <a href="STYLE_GUIDE.md">STYLE_GUIDE.md</a>. Self-contained: embedded font, no scripts,
  no external assets.
</footer>
</div>
"""


head_end = CONTENT.index("</style>") + len("</style>")
FULL = (
    "<!DOCTYPE html>\n<html lang=\"en\"><head>\n<meta charset=\"utf-8\">\n"
    "<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n"
    + CONTENT[:head_end]
    + "\n</head><body>\n"
    + CONTENT[head_end:]
    + "\n</body></html>\n"
)

OUT.write_text(FULL)
print(f"validated {ASSERTED_LINES} exact-width terminal rows")
print(
    "validated proposal screens: "
    + ", ".join(f"{width}x{height}" for width, height in PROPOSAL_CONSTANTS["screen_sizes"])
)
print(f"wrote {OUT} ({len(FULL)} bytes)")
