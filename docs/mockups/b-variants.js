// Round 3 of #251: five layouts of direction B (cards). ?b=1..5 & v=today|time.
// Same demo data as mock.js; b-variants.css does the styling, colours only from theme.css.
const q = new URLSearchParams(location.search);
const b = q.get("b") ?? "1";
let v = q.get("v") ?? "today"; // r4.js switches it to reuse the views below
document.documentElement.dataset.b = b;

const kbd = (k) => `<kbd>${k}</kbd>`;
const tags = (t = []) => t.map((x) => `<span class="tag">${x}</span>`).join("");
const card = (title, body, { cls = "", hint = "", count = "" } = {}) =>
  `<section class="card ${cls}"><h2>${title}${count !== "" ? `<span class="count">${count}</span>` : ""}${hint ? `<span class="hint">${kbd(hint)}</span>` : ""}</h2>${body}</section>`;

// Tasks: scheduled today, overdue/deadlines, plus a few DOING/DONE ones for the board.
const sched = [
  { t: "Review the release checklist", tags: ["work"], time: "14:00", file: "harbor" },
  { t: "Book a dentist appointment", tags: ["home"], file: "home" },
  { t: "Stretch for ten minutes", habit: true, file: "home" },
  { kw: "NEXT", t: "Write the partner onboarding guide", tags: ["work", "docs"], file: "orchard" },
];
const late = [
  { pri: "B", t: "Fix the crash when a photo upload is cancelled", tags: ["work", "bug"], late: "Scheduled 1d ago", file: "harbor" },
  { pri: "A", t: "Renew the bike insurance", tags: ["home"], late: "Due in 5d", file: "home" },
  { pri: "A", t: "Load test the rate limiter", tags: ["work"], late: "Due in 2d", file: "orchard" },
];
const habit = "111110111111101111".split("").map((c) => `<i class="${c === "1" ? "y" : "n"}"></i>`).join("");
const meta = (r) => `<span class="meta">${r.habit ? `<span class="habit">${habit}</span><span>3 in a row</span>` : ""}${r.time ? `<span class="time">${r.time}</span>` : ""}${r.late ? `<span class="late">${r.late}</span>` : ""}${r.file ? `<span class="file">${r.file}</span>` : ""}</span>`;
const row = (r, sel) => `<li class="task${sel ? " sel" : ""}"><span class="box"></span>${r.kw ? `<span class="kw">${r.kw}</span>` : ""}${r.pri ? `<span class="pri p${r.pri}">${r.pri}</span>` : ""}<span class="title">${r.t}</span>${tags(r.tags)}${meta(r)}</li>`;
const now = `<li class="now"><span>now 15:58</span></li>`;
const list = (rows, { sel = false, nowAt = -1, compact = false } = {}) =>
  `<ul class="tasks${compact ? " compact" : ""}">${rows.map((r, i) => row(r, sel && i === 0) + (i === nowAt ? now : "")).join("")}</ul>`;

// Time: today's sessions, week, projects. Colour per project.
const pc = { Orchard: "var(--h3)", Harbor: "var(--link)", Internal: "var(--h2)" };
const sess = [["09:28", "12:13", "Orchard", "Load test plan", "2h 45m"], ["12:58", "13:28", "Internal", "Team sync", "30m"], ["13:38", "now", "Harbor", "Photo upload fixes", "2h 20m"]];
const week = [["Mon", "8h 05m", 97], ["Tue", "8h 15m", 100], ["Wed", "8h 05m", 97], ["Thu", "8h 15m", 100], ["Fri", "5h 36m", 68], ["Sat", "–", 0], ["Sun", "–", 0]];
const projs = [["Orchard", "ORC-100", "17h 45m", "17.75 h", 47], ["Harbor", "HRB-210", "13h 30m", "13.50 h", 35], ["Internal", "INT", "4h 40m", "4.67 h", 12]];
// Week as blocks [from, to (hours), project] for the timeline views.
const blocks = {
  Mon: [[8.5, 12, "Orchard"], [12.6, 17.2, "Harbor"]],
  Tue: [[8, 11.5, "Harbor"], [12, 16.75, "Orchard"]],
  Wed: [[8.5, 9, "Internal"], [9, 12, "Orchard"], [13, 17.6, "Harbor"]],
  Thu: [[8, 12.25, "Orchard"], [13, 14, "Internal"], [14, 17, "Harbor"]],
  Fri: [[9.47, 12.22, "Orchard"], [12.97, 13.47, "Internal"], [13.63, 15.97, "Harbor"]],
};
const H0 = 8, H1 = 18, pct = (h) => ((h - H0) / (H1 - H0)) * 100;

const dot = `<span class="dot"></span>`;
const running = (compact = false) => `<section class="card running${compact ? " compact" : ""}">
  <div class="who">${dot}<div><small>Tracking since 13:38</small><h3>Harbor</h3><p>Fix the crash when a photo upload is cancelled</p></div>
    <div class="total"><b>${compact ? "2h 20m" : "5h 36m"}</b><small>${compact ? "this session" : "today"}</small></div></div>
  <div class="acts"><button>${kbd("p")}Pause</button><button>${kbd("c")}Switch</button>${compact ? "" : `<button>${kbd("a")}Started earlier…</button>`}<button class="primary">${kbd("o")}Stop</button></div></section>`;
const chip = `<span class="clockchip">${dot}<b>Harbor</b><span class="mono">2h 20m</span>${kbd("I")}</span>`;
const sessions = (hint = "") => card("Today", `<table>${sess.map(([a, z, p, n, d]) =>
  `<tr class="${z === "now" ? "live" : ""}"><td class="mono">${a}–${z}</td><td><i class="sw" style="background:${pc[p]}"></i><b>${p}</b> <span class="muted">${n}</span></td><td class="mono r">${d}</td></tr>`).join("")}</table>`, { hint, cls: "sessions" });
const bars = (h = 100) => `<div class="bars" style="--bh:${h}px">${week.map(([dn, t, p]) =>
  `<div class="day${dn === "Fri" ? " cur" : ""}"><div class="bar"><i style="height:${p}%"></i></div><span class="h">${t}</span><span class="dn">${dn}</span></div>`).join("")}</div>`;
const flex = `<div class="flex"><div><small>Flex balance</small><b class="neg">-3h 25m</b></div><div><small>This week</small><b class="neg">-4h 05m</b></div></div>`;
const projTable = `<table><tr><th>Project</th><th>Code</th><th class="r">Worked</th><th class="r">Billable</th></tr>${projs.map(([p, c, w, bl, s]) =>
  `<tr><td><i class="sw" style="background:${pc[p]}"></i>${p}<span class="share" style="width:${s}%;background:${pc[p]}"></span></td><td class="mono muted">${c}</td><td class="mono r">${w}</td><td class="mono r">${bl}</td></tr>`).join("")}</table>`;
const projShares = `<ul class="shares">${projs.map(([p, , w, , s]) => `<li><span><i class="sw" style="background:${pc[p]}"></i>${p}</span><span class="mono">${w}</span><div class="track"><i style="width:${s * 2}%;background:${pc[p]}"></i></div></li>`).join("")}</ul>`;
const reports = `<div class="btns">${[["t", "Day report"], ["w", "Week report"], ["e", "Export CSV…"], ["E", "Export report…"]].map(([k, l]) => `<button>${kbd(k)}${l}</button>`).join("")}</div>`;
const journal = card("Journal", `<p class="jtext">Shipped the sync fix this morning. The onboarding guide needs a section on API keys; ask Sam about the staging quota.</p><span class="muted small">journal/2026-10-02.org · 3 entries</span>`, { hint: "J", cls: "journal" });
const habits = card("Habits", `<div class="habitbig"><b>3</b><span class="muted">days in a row</span></div><span class="habit wide">${habit}</span><span class="muted small">Stretch for ten minutes · 15 of 18 days</span>`, { hint: "␣ h" });
const hints = `<footer class="hints">${[["x", "Done"], ["s", "Schedule"], ["d", "Due"], ["p", "Priority"], ["t", "Status"], ["#", "Tags"], ["I", "Track time"], ["↵", "Open"]]
  .map(([k, l]) => `<span>${kbd(k)}${l}</span>`).join("")}<span class="more">${kbd("?")}More</span></footer>`;
const head = (title, sub, controls = "") => `<header class="vhead"><div><h1>${title}</h1><div class="sub">${sub}</div></div><div class="controls">${controls}</div></header>`;
const dayControls = `<span class="seg"><b>Day</b><span>Week</span></span><button>‹</button><button>Today</button><button>›</button><button class="primary">+ New task</button>`;
const profile = `<span class="muted">Profile</span><button>Work ▾</button>`;
const navItems = [["◎", "Today", "1"], ["☑", "Tasks", "2"], ["↓", "Inbox", "3"], ["✎", "Journal", "J"], ["◷", "Time", "4"], ["▤", "Projects", "5"]];
const isOn = (l) => l === (v === "time" ? "Time" : "Today");
const pills = `<nav class="pills">${navItems.map(([, l, k]) => `<a class="${isOn(l) ? "on" : ""}">${l}${isOn(l) ? "" : kbd(k)}</a>`).join("")}</nav>`;
const search = `<button class="cmdk"><span>⌕</span><span class="lbl">Search or run a command…</span>${kbd("Ctrl K")}</button>`;

// B1: icon rail + one centred column of cards.
function b1() {
  const rail = `<nav class="rail"><span class="logo">m</span>${navItems.map(([i, l, k]) => `<a class="${isOn(l) ? "on" : ""}"><span class="ico">${i}</span><span class="lbl">${l}</span></a>`).join("")}
    <span class="grow"></span><a><span class="ico">⌕</span><span class="lbl">Ctrl K</span></a><a class="timer">${dot}<span class="lbl mono">2h 20m</span></a></nav>`;
  const body = v === "time"
    ? head("Time", "Week 40 · 38h 16m of 40h", profile) + running() +
      `<div class="two">${sessions("␣ t")}${card("This week", bars(92) + flex, { hint: "w" })}</div>` + card("Projects this week", projTable, { hint: "␣ p", cls: "projects" })
    : head("Today", "Friday, October 2", dayControls) +
      card("Scheduled", list(sched, { sel: true, nowAt: 0 }), { count: 4, hint: "␣ a" }) +
      card("Overdue and deadlines", list(late), { count: 3, hint: "␣ o" }) + `<div class="two">${journal}${habits}</div>`;
  return `<div class="b1">${rail}<main><div class="col">${body}</div>${v === "time" ? "" : hints}</main></div>`;
}

// B2: no sidebar, slim top bar, dashboard grid of cards with spans.
function b2() {
  const top = `<header class="topbar">${search}${pills}<span class="grow"></span>${chip}</header>`;
  const grid = v === "time"
    ? `${running().replace('class="card running', 'class="card running s8')}
       ${card("Worked today", `<div class="big mono">5h 36m</div><div class="progress"><i style="width:70%"></i></div><span class="muted small">of 8h 00m · 2h 24m to go</span>`, { cls: "s4" })}
       ${sessions("␣ t").replace('class="card sessions', 'class="card sessions s5')}
       ${card("This week", bars(110), { cls: "s7", hint: "w" })}
       ${card("Projects this week", projTable, { cls: "s8 projects", hint: "␣ p" })}
       ${card("Balance", flex + reports, { cls: "s4" })}`
    : `<div class="greet s12"><h1>Good afternoon</h1><span class="muted">Friday, October 2</span><span class="stat">4 scheduled</span><span class="stat late">3 overdue</span><span class="stat">5h 36m tracked</span><span class="grow"></span><button class="primary">+ New task</button></div>
       ${card("Scheduled", list(sched, { sel: true, nowAt: 0 }), { cls: "s7 r2", count: 4, hint: "␣ a" })}
       ${running(true).replace('class="card running', 'class="card running s5')}
       ${card("Overdue and deadlines", list(late, { compact: true }), { cls: "s5", count: 3, hint: "␣ o" })}
       ${journal.replace('class="card journal', 'class="card journal s4')}${habits.replace('class="card ', 'class="card s4 ')}
       ${card("This week", bars(64), { cls: "s4", hint: "4" })}`;
  return `<div class="b2">${top}<main class="grid">${grid}</main></div>`;
}

// B3: one big focus card, collapsible right panel of small cards.
function b3() {
  const crumbs = `<header class="crumbs"><span class="ico">≡</span>${pills}<span class="grow"></span>${search}</header>`;
  const focus = v === "time"
    ? `<section class="card focus">${head("Time", "Week 40 · 38h 16m of 40h", profile)}
        <div class="dayline">${["09", "11", "13", "15", "17"].map((h) => `<span style="left:${pct(+h)}%">${h}</span>`).join("")}
          ${blocks.Fri.map(([a, z, p]) => `<i style="left:${pct(a)}%;width:${pct(z) - pct(a)}%;background:${pc[p]}"></i>`).join("")}<b class="nowmark" style="left:${pct(15.97)}%"></b></div>
        <h2>Sessions today</h2>${sessions().replace(/<section class="card sessions"><h2>Today<\/h2>|<\/section>/g, "")}
        <h2>This week</h2>${bars(120)}</section>`
    : `<section class="card focus">${head("Today", "Friday, October 2", dayControls)}
        <h2>Scheduled <span class="count">4</span></h2>${list(sched, { sel: true, nowAt: 0 })}
        <h2>Overdue and deadlines <span class="count">3</span></h2>${list(late)}${hints}</section>`;
  const side = v === "time"
    ? running(true) + card("Balance", flex, { hint: "b" }) + card("Projects this week", projShares, { hint: "␣ p" }) + card("Reports", reports)
    : running(true) + card("Up next", `<div class="next"><span class="time mono">14:00</span><div><b>Review the release checklist</b><span class="muted small">harbor · in 2h 02m</span></div></div>`, { hint: "n" }) +
      journal + card("Linked to Harbor", `<ul class="links"><li>sync-design</li><li>reading</li><li>release-2.4</li></ul>`, { hint: "␣ b" });
  return `<div class="b3">${crumbs}<main>${focus}<aside class="panel"><div class="ptitle"><span>Panel</span>${kbd("Ctrl \\")}</div>${side}</aside></main></div>`;
}

// B4: slim top bar, Today as an hour timeline, Time as a week timeline.
function b4() {
  const top = `<header class="topbar"><span class="brand">margin</span>${pills}<span class="grow"></span>${search}${chip}</header>`;
  const hours = Array.from({ length: H1 - H0 + 1 }, (_, i) => H0 + i);
  const at = (a, z) => `top:${pct(a)}%;height:${pct(z) - pct(a)}%`;
  const tl = `<div class="tl">${hours.map((h) => `<span class="hr" style="top:${pct(h)}%">${String(h).padStart(2, "0")}:00</span>`).join("")}
    ${blocks.Fri.map(([a, z, p], i) => `<div class="blk" style="${at(a, z)};--c:${pc[p]}"><b>${p}</b><small>${sess[i][3]} · ${sess[i][4]}</small></div>`).join("")}
    <div class="evt" style="${at(14, 14.5)}"><span class="box"></span><b>Review the release checklist</b><span class="tag">work</span></div>
    <div class="evt cal" style="${at(16.5, 17)}"><b>Dentist call</b><small>16:30 · calendar</small></div>
    <div class="nowline" style="top:${pct(15.97)}%"><span>15:58</span></div></div>`;
  const main = v === "time"
    ? `<div class="strip">${running(true)}${card("Week 40", `<div class="big mono">38h 16m</div><span class="muted small">of 40h 00m</span>`)}${card("Balance", flex)}</div>
       ${card("Week timeline", `<div class="gantt"><div class="ghead"><span></span><div class="ticks">${hours.map((h) => `<span style="left:${pct(h)}%">${String(h).padStart(2, "0")}</span>`).join("")}</div><span></span></div>
         ${Object.entries(blocks).map(([d, bs]) => `<div class="grow-row${d === "Fri" ? " cur" : ""}"><span class="dn">${d}</span><div class="lane">${bs.map(([a, z, p]) => `<i style="left:${pct(a)}%;width:${pct(z) - pct(a)}%;background:${pc[p]}"></i>`).join("")}${d === "Fri" ? `<b class="nowmark" style="left:${pct(15.97)}%"></b>` : ""}</div><span class="mono r">${week.find((w) => w[0] === d)[1]}</span></div>`).join("")}
         <div class="legend">${Object.entries(pc).map(([p, c]) => `<span><i class="sw" style="background:${c}"></i>${p}</span>`).join("")}</div></div>`, { cls: "wide", hint: "w" })}
       ${card("Projects this week", projTable, { cls: "projects", hint: "␣ p" })}`
    : `${card("Friday, October 2", tl, { cls: "timeline", hint: "␣ a" })}
       <div class="stack">${card("Anytime today", list(sched.slice(1), { compact: true }), { count: 3, hint: "␣ t" })}${card("Overdue and deadlines", list(late, { compact: true }), { count: 3, hint: "␣ o" })}${journal}</div>`;
  return `<div class="b4">${top}<main class="v-${v}">${main}</main></div>`;
}

// B5: header strip (clock + agenda), tasks as board columns; Time as project lanes.
function b5() {
  const agenda = `<div class="agenda">${[["09:28", "Orchard · load test plan", "done"], ["13:38", "Harbor · tracking", "live"], ["14:00", "Release checklist", ""], ["16:30", "Dentist call", ""]]
    .map(([t, l, c]) => `<span class="ag ${c}"><b class="mono">${t}</b>${l}</span>`).join("")}</div>`;
  const strip = `<header class="strip"><div class="date"><b>Fri 2</b><span class="muted">October · week 40</span></div>${agenda}<span class="grow"></span>${chip}${search}</header>`;
  const kc = (r, extra = "") => `<div class="kcard${extra}"><div class="kt">${r.pri ? `<span class="pri p${r.pri}">${r.pri}</span>` : ""}${r.t}</div><div class="kmeta">${tags(r.tags)}${r.habit ? `<span class="habit">${habit}</span>` : ""}${r.time ? `<span class="time mono">${r.time}</span>` : ""}${r.late ? `<span class="late">${r.late}</span>` : ""}<span class="file">${r.file}</span></div>${r.track ? `<div class="track-on">${dot}tracking 2h 20m</div>` : ""}</div>`;
  const colm = (name, n, cards, key, cls = "") => `<section class="kcol ${cls}"><h2>${name}<span class="count">${n}</span><span class="hint">${kbd(key)}</span></h2>${cards}<button class="add">+ Add</button></section>`;
  const board = v === "time"
    ? Object.keys(pc).map((p, i) => colm(p, projs[i][2], `<div class="ptot"><div class="track"><i style="width:${projs[i][4] * 2}%;background:${pc[p]}"></i></div><span class="muted small mono">${projs[i][1]} · ${projs[i][3]} billable</span></div>` +
        Object.entries(blocks).reverse().flatMap(([d, bs]) => bs.filter((x) => x[2] === p).map(([a, z]) => {
          const f = (h) => `${String(Math.floor(h)).padStart(2, "0")}:${String(Math.round((h % 1) * 60)).padStart(2, "0")}`, m = Math.round((z - a) * 60);
          const live = d === "Fri" && p === "Harbor";
          return `<div class="kcard${live ? " live" : ""}" style="--c:${pc[p]}"><div class="kt">${d} · ${f(a)}–${live ? "now" : f(z)}</div><div class="kmeta"><span class="mono">${m < 60 ? "" : `${Math.floor(m / 60)}h `}${String(m % 60).padStart(m < 60 ? 1 : 2, "0")}m</span>${live ? `<span class="track-on">${dot}running</span>` : ""}</div></div>`;
        })).slice(0, 4).join(""), String(i + 1), "proj")).join("") +
      `<section class="kcol side">${card("This week", bars(80) + flex, { hint: "w" })}${card("Reports", reports)}</section>`
    : colm("Todo", 3, [sched[1], sched[2], late[1]].map((r) => kc(r)).join(""), "t") +
      colm("Next", 2, [sched[3], late[2]].map((r) => kc(r)).join(""), "n") +
      colm("Doing", 2, kc({ ...late[0], track: true }, " sel") + kc(sched[0]), "g", "doing") +
      colm("Done", 3, [{ t: "Update the changelog", tags: ["work"], file: "harbor" }, { t: "Reply to the design review", tags: ["work"], file: "orchard" }, { t: "Water the plants", tags: ["home"], file: "home" }].map((r) => kc(r, " done")).join(""), "x", "donecol");
  return `<div class="b5">${strip}${v === "time" ? `<div class="subhead">${head("Time", "Week 40 · sessions by project", profile)}</div>` : `<div class="subhead">${head("Tasks", "Today's board · drag or press a column key", `${pills}<button class="primary">+ New task</button>`)}</div>`}<main class="board v-${v}">${board}</main></div>`;
}

if (!window.R4) document.body.innerHTML = { 1: b1, 2: b2, 3: b3, 4: b4, 5: b5 }[b]();
