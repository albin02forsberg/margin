// Round 4 of #251: B1 (rail + cards), B2 (dashboard grid) and B5 (board) on more pages, B1's shell with
// B5's board as Tasks (s=mix), and motion demos (?demo=…; &t=ms pauses every animation at that time, for render.sh).
// Reuses the data and helpers of b-variants.js (loaded first; window.R4 stops it rendering itself).
const S = q.get("s") ?? "1", P = q.get("p") ?? "editor", DEMO = q.get("demo"), T = q.get("t");
const frag = (html, sel) => { const t = document.createElement("template"); t.innerHTML = html; return t.content.querySelector(sel); };
const view = (name, fn) => { const old = v; v = name; const r = fn(); v = old; return r; };

/* Shells: B1 rail, B2 top bar + grid, B5 agenda strip + board. */
const NAV = [["◎", "Today", "1", "today"], ["☑", "Tasks", "2", "tasks"], ["✎", "Notes", "P", "editor"], ["◷", "Time", "4", "time"], ["▤", "Projects", "5", "projects"], ["◈", "Graph", "G", "graph"]];
const rail4 = (p) => `<nav class="rail"><span class="logo">m</span>${NAV.map(([i, l, , id]) => `<a class="${p === id ? "on" : ""}" data-p="${id}"><span class="ico">${i}</span><span class="lbl">${l}</span></a>`).join("")}
  <span class="grow"></span><a><span class="ico">⌕</span><span class="lbl">Ctrl K</span></a><a class="${p === "settings" ? "on" : ""}"><span class="ico">⚙</span><span class="lbl">Settings</span></a><a class="timer">${dot}<span class="lbl mono">2h 20m</span></a></nav>`;
const pills4 = (p) => `<nav class="pills">${NAV.map(([, l, k, id]) => `<a class="${p === id ? "on" : ""}">${l}${p === id ? "" : kbd(k)}</a>`).join("")}</nav>`;
const shell1 = (p, body, { wide = false, extra = "" } = {}) => `<div class="b1">${rail4(p)}<main><div class="col${wide ? " wide" : ""}">${body}</div>${extra}</main></div>`;
const shell2 = (p, body) => `<div class="b2">${`<header class="topbar">${search}${pills4(p)}<span class="grow"></span>${chip}<button class="gear">⚙</button></header>`}<main class="grid g4">${body}</main></div>`;
const strip5 = `<header class="strip"><div class="date"><b>Fri 2</b><span class="muted">October · week 40</span></div><div class="agenda">${[["09:28", "Orchard · load test plan", "done"], ["13:38", "Harbor · tracking", "live"], ["14:00", "Release checklist", ""], ["16:30", "Dentist call", ""]]
  .map(([t, l, c]) => `<span class="ag ${c}"><b class="mono">${t}</b>${l}</span>`).join("")}</div><span class="grow"></span>${chip}${search}<button class="gear">⚙</button></header>`;
const shell5 = (p, title, sub, controls, body, cls = "") => `<div class="b5">${strip5}<div class="subhead">${head(title, sub, pills4(p) + controls)}</div><main class="r5 ${cls}">${body}</main></div>`;
const chips = (xs, on = 0) => `<span class="chips">${xs.map((x, i) => `<span class="${i === on ? "on" : ""}">${x}</span>`).join("")}</span>`;
const seg = (xs, on = 0) => `<span class="seg">${xs.map((x, i) => i === on ? `<b>${x}</b>` : `<span>${x}</span>`).join("")}</span>`;

/* Board cards (B5 and the mix). */
const kc = (r, extra = "") => `<div class="kcard${extra}"><div class="kt">${r.pri ? `<span class="pri p${r.pri}">${r.pri}</span>` : ""}${r.t}</div><div class="kmeta">${tags(r.tags)}${r.habit ? `<span class="habit">${habit}</span>` : ""}${r.time ? `<span class="time mono">${r.time}</span>` : ""}${r.late ? `<span class="late">${r.late}</span>` : ""}${r.file ? `<span class="file">${r.file}</span>` : ""}</div>${r.track ? `<div class="track-on">${dot}tracking 2h 20m</div>` : ""}</div>`;
const kcol = (name, n, cards, key, cls = "") => `<section class="kcol ${cls}"><h2>${name}<span class="count">${n}</span><span class="hint">${kbd(key)}</span></h2>${cards}<button class="add">+ Add</button></section>`;
const done3 = [{ t: "Update the changelog", tags: ["work"], file: "harbor" }, { t: "Reply to the design review", tags: ["work"], file: "orchard" }, { t: "Water the plants", tags: ["home"], file: "home" }];
const roadmap = { t: "Draft the Q4 roadmap", tags: ["work"], file: "orchard" };
const board = { // all open tasks by state
  Todo: [sched[1], late[1], roadmap, sched[2]], Next: [sched[3], late[2]], Doing: [{ ...late[0], track: true }, sched[0]], Done: done3,
};
const boardCols = (b = board) => kcol("Todo", b.Todo.length, b.Todo.map((r) => kc(r)).join(""), "t") + kcol("Next", b.Next.length, b.Next.map((r) => kc(r)).join(""), "n") +
  kcol("Doing", b.Doing.length, b.Doing.map((r, i) => kc(r, i ? "" : " sel")).join(""), "g", "doing") + kcol("Done", b.Done.length, b.Done.map((r) => kc(r, " done")).join(""), "x", "donecol");
const boardHints = `<footer class="hints">${[["t n g x", "Move to column"], ["H L", "Move left / right"], ["J K", "Up / down"], ["↵", "Open"], ["I", "Track time"]]
  .map(([k, l]) => `<span>${kbd(k)}${l}</span>`).join("")}</footer>`;

/* Editor: harbor.org as the editor shows it (org syntax, Vim status line). */
const star = (n) => `<span class="stars">${"*".repeat(n)}</span> `;
const otags = (...t) => `<span class="otags">${t.map((x) => `:${x}:`).join("")}</span>`;
const lnk = (s) => `<span class="olink">${s}</span>`;
const odate = (s) => `<span class="odate">&lt;${s}&gt;</span>`;
const ln = (cls, html = "") => `<div class="ln ${cls}">${html || "&nbsp;"}</div>`;
const note = `<div class="org">
  ${ln("meta", `<span class="okw">#+title:</span> Harbor`)}${ln("meta", `<span class="okw">#+filetags:</span> :work:`)}${ln("blank")}
  ${ln("h1", star(1) + "Photo upload fixes" + otags("work"))}${ln("plan", `<span class="okw">DEADLINE:</span> ${odate("2026-10-07 Wed")}`)}
  ${ln("h2 cur", star(2) + `<span class="otodo">TODO</span> <span class="opri">[#B]</span> Fix the crash when a photo <span class="cursor">u</span>pload is cancelled` + otags("bug"))}
  ${ln("plan", `<span class="okw">SCHEDULED:</span> ${odate("2026-10-01 Thu")}`)}${ln("plan", `<span class="fold">:LOGBOOK:…</span> <span class="muted">3 entries · 4h 10m</span>`)}
  ${ln("h2", star(2) + `<span class="onext">NEXT</span> Retry uploads on flaky networks`)}
  ${ln("h2 done", star(2) + `<span class="odone">DONE</span> Show upload progress per photo`)}${ln("blank")}
  ${ln("h1", star(1) + "Release 2.4")}
  ${ln("p", `The checklist lives in ${lnk("release-2.4")}; the sync notes are in ${lnk("sync-design")}.`)}
  ${ln("li", `<span class="cbx on">✓</span> Freeze strings for translation`)}${ln("li", `<span class="cbx"></span> Review the release checklist ${odate("2026-10-02 Fri 14:00")}`)}${ln("li", `<span class="cbx"></span> Tag and publish the build`)}${ln("blank")}
  ${ln("h1", star(1) + "Notes")}
  ${ln("p", "Uploads retry three times with backoff (1 s, 4 s, 16 s). A cancelled upload has to delete its temp file before the retry timer fires, or the next try reads a half-written file.")}
  ${ln("p", `Background: ${lnk("reading")} has the article on resumable uploads.`)}
</div>`;
const vimline = `<div class="vimline"><span class="mode">NORMAL</span><span>harbor.org</span><span class="muted crumb">Photo upload fixes › Fix the crash…</span><span class="grow"></span><span class="mono muted">6:42</span><span class="saved">Saved</span></div>`;
const etabs = `<div class="etabs">${["harbor", "orchard", "sync-design", "journal · Oct 2"].map((t, i) => `<span class="${i ? "" : "on"}">${t}${i ? "" : `<i>×</i>`}</span>`).join("")}<span class="grow"></span>${kbd("Ctrl P")}</div>`;
const editorCard = (cls = "") => `<section class="card editor ${cls}">${etabs}${note}${vimline}</section>`;
const outline = card("Outline", `<ul class="outline"><li class="o1">Photo upload fixes</li><li class="o2 on">Fix the crash when a photo…</li><li class="o2">Retry uploads on flaky networks</li><li class="o2 dn">Show upload progress</li><li class="o1">Release 2.4</li><li class="o1">Notes</li></ul>`, { hint: "␣ n o" });
const backlinks = card("Linked here", `<ul class="links"><li>sync-design</li><li>reading</li><li>release-2.4</li><li>journal · Oct 1</li></ul>`, { count: 4, hint: "␣ n b" });
const noteTasks = card("Tasks in this note", list([{ pri: "B", t: "Fix the crash when a photo upload is cancelled", late: "1d ago" }, { kw: "NEXT", t: "Retry uploads on flaky networks" }, { t: "Review the release checklist", time: "14:00" }], { compact: true }), { count: 3 });
const timeOn = card("Time on Harbor", `<div class="big mono">13h 30m</div><span class="muted small">this week · HRB-210 · 2h 20m now</span>${bars(46)}`, { hint: "I" });

/* Tasks / agenda */
const agendaDays = [["Fri", "2", "today", [sched[0], sched[1], { ...late[0], late: "1d ago" }]], ["Sat", "3", "", [{ t: "Water the plants", tags: ["home"], file: "home" }]],
  ["Mon", "5", "", [roadmap, { t: "Team sync", time: "10:00", file: "internal" }]], ["Tue", "6", "", [{ ...late[2], late: "deadline" }]], ["Wed", "7", "", [{ t: "Photo upload fixes", late: "deadline", file: "harbor" }, { ...late[1], late: "deadline" }]]];
const agenda = (n = 5) => `<div class="agdays">${agendaDays.slice(0, n).map(([d, n, c, rs]) => `<div class="agday ${c}"><div class="dd"><span>${d}</span><b>${n}</b></div>${list(rs, { compact: true })}</div>`).join("")}</div>`;
const inbox = card("Inbox", list([{ t: "Look into the flaky upload test", tags: ["work"] }, { t: "Gift for Mia's birthday", tags: ["home"] }, { t: "Read the CRDT survey", tags: ["reading"] }], { compact: true }), { count: 3, hint: "␣ v i" });
const unscheduled = card("No date", `${[["orchard", [sched[3], roadmap]], ["home", [late[1]]]].map(([f, rs]) => `<div class="fgroup"><span class="muted small">${f}.org</span>${list(rs, { compact: true })}</div>`).join("")}`, { count: 3, hint: "␣ v a" });
const byTag = card("By tag", `<ul class="shares">${[["work", 8, 80], ["home", 4, 40], ["docs", 1, 10], ["bug", 1, 10]].map(([t, n, w]) => `<li><span class="tag">${t}</span><span class="mono">${n}</span><div class="track"><i style="width:${w}%;background:var(--link)"></i></div></li>`).join("")}</ul>`);
const doneWeek = card("Done this week", `<div class="habitbig"><b>9</b><span class="muted">tasks · 3 today</span></div>`);
const views = card("Saved views", `<ul class="links plain"><li>Work this week <span class="count">6</span></li><li>Waiting on others <span class="count">2</span></li><li>Someday <span class="count">5</span></li></ul>`, { hint: "␣ v v" });
const taskControls = chips(["All", "work", "home", "docs"]) + seg(["List", "Board"]) + `<button class="primary">+ New task</button>`;

/* Projects */
const P4 = [["Orchard", "ORC-100", "17h 45m", "64h 10m", 47, 4], ["Harbor", "HRB-210", "13h 30m", "41h 05m", 35, 3], ["Internal", "INT", "4h 40m", "18h 20m", 12, 1]];
pc.Garden = "var(--h4)";
const pcard = ([n, c, w, m, s, open], cls = "") => `<section class="card pcard ${cls}" style="--c:${pc[n]}"><h2><i class="sw" style="background:${pc[n]}"></i>${n}<span class="code mono">${c}</span><span class="hint">${kbd("I")}</span></h2>
  <div class="big mono">${w}</div><span class="muted small">this week · ${m} this month</span><div class="track"><i style="width:${s * 2}%;background:${pc[n]}"></i></div>
  <div class="pmeta"><span>${open} open tasks</span><span class="olink">${n.toLowerCase()}.org</span><span class="grow"></span><button>Track</button></div></section>`;
const hiddenCard = `<section class="card pcard hidden"><h2><i class="sw" style="background:${pc.Garden}"></i>Garden<span class="badge">hidden</span></h2><span class="muted small">2h 00m in September · not offered when tracking</span><div class="pmeta"><span class="grow"></span><button>Unhide</button></div></section>`;
const inp = (s, cls = "") => `<span class="inp ${cls}">${s}</span>`;
const timeOff = card("Time off", `<table>${[["Vacation", "VAC"], ["Sick leave", "SICK"], ["Parental leave", "PAR"]].map(([l, c]) => `<tr><td>${l}</td><td class="r">${inp(c, "mono")}</td></tr>`).join("")}</table><span class="muted small">Goes into the CSV export as its own rows under this code.</span>`, { hint: "␣ p o" });
const allProjects = card("All projects", `<table><tr><th>Project</th><th>Export code</th><th class="r">This week</th><th class="r">Month</th><th class="r">Hidden</th></tr>${[...P4, ["Garden", "", "–", "2h 00m", 0, 0]].map(([n, c, w, m]) =>
  `<tr class="${n === "Garden" ? "dim" : ""}"><td><i class="sw" style="background:${pc[n]}"></i>${n}</td><td>${inp(c || "&nbsp;", "mono")}</td><td class="mono r">${w}</td><td class="mono r">${m}</td><td class="r"><span class="tog${n === "Garden" ? " on" : ""}"><i></i></span></td></tr>`).join("")}</table><span class="muted small">Enter in an export code saves its row.</span>`, { cls: "projects" });

/* Settings: the real groups and fields of src/lib/settings.ts */
const tog = (on) => `<span class="tog${on ? " on" : ""}"><i></i></span>`;
const SET = {
  Folders: [["Notes folder", inp("~/notes")], ["Time-tracking folder", inp("~/notes/.time")], ["Export folder", inp("~/Documents/margin")], ["Journal folder", inp("journal"), "relative to the notes folder"], ["Inbox file", inp("inbox.org"), "new tasks go here"]],
  Time: [["Expected hours per day", inp("8", "num")], ["Ask about idle time after (minutes)", inp("10", "num"), "0 = never"], ["Profiles", inp("Work<br>Side projects", "area"), "one per line; each has its own time log"]],
  "Reminders & desktop": [["Desktop reminders", tog(true)], ["Remind before timed entries (minutes)", inp("10", "num")], ["Warn about deadlines (days ahead)", inp("3", "num")], ["Quick-capture shortcut", inp("Super+Shift+N", "mono")], ["Closing the window while tracking hides it to the tray", tog(true)], ["Nightly updates", tog(false)]],
  Tasks: [["Open keywords", inp("TODO<br>NEXT<br>WAITING", "area mono")], ["Done keywords", inp("DONE<br>CANCELLED", "area mono")], ["Calendar feed (.ics)", inp("&nbsp;"), "empty = off"]],
  Integrations: [["Built-in activity watcher", tog(false), "stays on this computer"], ["ActivityWatch server", inp("http://localhost:5600", "mono")], ["Ollama server", inp("http://localhost:11434", "mono"), "local addresses only"]],
};
const field = ([l, c, h]) => `<div class="field"><div><span>${l}</span>${h ? `<small>${h}</small>` : ""}</div>${c}</div>`;
const setCard = (g, cls = "") => card(g, SET[g].map(field).join(""), { cls: `settings ${cls}`, count: SET[g].length });
const setHead = (sub = "config.toml · saved as you type") => head("Settings", sub, `<span class="filter">Filter settings ${kbd("/")}</span><button>Edit config.toml</button>`);

/* Command palette, over each shell's Today */
const cmd = (l, keys = "", sel = false, hl = "") => `<li class="${sel ? "sel" : ""}"><span>${hl ? l.replace(hl, `<mark>${hl}</mark>`) : l}</span><span class="keys">${keys.split(" · ").filter(Boolean).map(kbd).join("")}</span></li>`;
const palList = (groups) => groups.map(([g, items]) => `<div class="pgroup">${g}</div><ul>${items.join("")}</ul>`).join("");
const palDefault = palList([["Go to", [cmd("Go to Today", "Ctrl 1 · ␣ v t", true, "Go to"), cmd("Go to All tasks", "Ctrl 2 · ␣ v a", false, "Go to"), cmd("Go to Inbox", "Ctrl 3 · ␣ v i", false, "Go to"), cmd("Go to Time tracking", "Ctrl 4 · ␣ v c", false, "Go to")]],
  ["Notes", [cmd("Open note…", "Ctrl P · ␣ ."), cmd("Today's journal", "Ctrl J · ␣ j"), cmd("Show note graph", "␣ n g")]], ["Recent", [cmd("harbor.org"), cmd("sync-design.org")]]]);
const palTime = palList([["Commands", [cmd("Go to Time tracking", "Ctrl 4 · ␣ v c", true, "Time"), cmd("Time: day report", "t", false, "Time"), cmd("Time: week report", "w", false, "Time"), cmd("Time: export CSV…", "e", false, "Time"), cmd("Time: check log", "", false, "Time")]]]);
const palette = (query = "go to") => `<div class="pal-back"></div><div class="pal"><div class="pin"><span class="ico">⌕</span><span class="q">${query.split("").map((c) => `<span class="ch">${c === " " ? "&nbsp;" : c}</span>`).join("")}<span class="ph">Search or run a command…</span><span class="caret"></span></span>${kbd("Esc")}</div>
  <div class="plists"><div class="plist a">${palDefault}</div><div class="plist b">${palTime}</div><span class="palsel"></span></div>
  <div class="pfoot">${[["↑↓", "Select"], ["↵", "Run"], ["Tab", "Complete"], ["Esc", "Close"]].map(([k, l]) => `<span>${kbd(k)}${l}</span>`).join("")}</div></div>`;
const today1 = () => view("today", () => frag(b1(), ".col").innerHTML);
const todayHints = () => frag(b1(), ".hints").outerHTML;
const today2 = () => view("today", () => frag(b2(), ".grid").innerHTML);
const today5 = () => shell5("today", "Today", "Friday, October 2 · board", `<button class="primary">+ New task</button>`, frag(b5(), ".board").innerHTML, "board v-today");

/* Graph: notes and links; harbor is selected, its neighbours highlighted. */
const GN = { harbor: [500, 290, 1], orchard: [300, 200], "sync-design": [660, 160], reading: [720, 380], "release-2.4": [560, 470], internal: [220, 400], "journal · Oct 1": [380, 420], "journal · Oct 2": [420, 540],
  onboarding: [150, 250], "api-keys": [360, 80], ideas: [110, 120], "load-test": [200, 60], crdt: [860, 240], home: [880, 520], "garden-plan": [930, 420] };
const GE = [["harbor", "sync-design"], ["harbor", "reading"], ["harbor", "release-2.4"], ["harbor", "journal · Oct 1"], ["harbor", "orchard"], ["orchard", "sync-design"], ["orchard", "onboarding"], ["orchard", "api-keys"],
  ["onboarding", "api-keys"], ["ideas", "onboarding"], ["orchard", "load-test"], ["release-2.4", "journal · Oct 2"], ["internal", "journal · Oct 2"], ["reading", "sync-design"], ["reading", "crdt"], ["internal", "orchard"]];
const deg = (n) => GE.filter((e) => e.includes(n)).length;
const near = (n) => n === "harbor" || GE.some((e) => e.includes("harbor") && e.includes(n));
const graphSvg = `<svg class="gsvg" viewBox="0 0 1000 600" preserveAspectRatio="xMidYMid meet">${GE.map(([a, z]) => `<line class="${a === "harbor" || z === "harbor" ? "hot" : ""}" x1="${GN[a][0]}" y1="${GN[a][1]}" x2="${GN[z][0]}" y2="${GN[z][1]}"/>`).join("")}
  ${Object.entries(GN).map(([n, [x, y, f]]) => `<g class="gn${f ? " focus" : near(n) ? " near" : ""}${deg(n) ? "" : " orphan"}"><circle cx="${x}" cy="${y}" r="${7 + deg(n) * 2.2}"/><text x="${x}" y="${y + 18 + deg(n) * 2.2}">${n}</text></g>`).join("")}</svg>`;
const graphCard = (cls = "") => `<section class="card graph ${cls}"><h2>Note graph<span class="hint">${seg(["Local", "All notes"], 1)}</span></h2>${graphSvg}<div class="glegend"><span><i class="gl gl-sel"></i>selected</span><span><i class="gl gl-near"></i>linked</span><span><i class="gl"></i>other</span><span class="grow"></span><span class="muted">scroll to zoom · drag to pan · ↵ open</span></div></section>`;
const selNote = card("harbor.org", `<p class="jtext">Photo upload fixes, Release 2.4, notes on retries.</p><div class="flex"><div><small>Links</small><b>5</b></div><div><small>Open tasks</small><b>3</b></div></div><button>${kbd("↵")}Open note</button>`, { hint: "␣ n g" });
const orphans = card("Not linked", `<ul class="links"><li>home</li><li>garden-plan</li></ul><span class="muted small">2 notes with no links in or out</span>`, { count: 2 });
const gfilters = card("Show", `${[["Journal entries", true], ["Tags as nodes", false], ["Unlinked notes", true]].map(([l, on]) => `<div class="field"><div><span>${l}</span></div>${tog(on)}</div>`).join("")}`);

/* Pages per shell */
const twoCol = (main, side) => `<div class="split">${main}<div class="stack">${side}</div></div>`;
const PAGES = {
  editor: {
    1: () => shell1("editor", twoCol(editorCard(), outline + backlinks + timeOn), { wide: true }),
    2: () => shell2("editor", `${editorCard("s8 r3")}${outline.replace('class="card ', 'class="card s4 ')}${backlinks.replace('class="card ', 'class="card s4 ')}${noteTasks.replace('class="card ', 'class="card s4 ')}`),
    5: () => shell5("editor", "harbor.org", "Notes · edited 2 min ago", `<button>${kbd("␣ n b")}Links</button>`,
      `${editorCard()}<div class="kside">${kcol("In this note", 3, [kc({ pri: "B", t: "Fix the crash when a photo upload is cancelled", late: "1d ago", track: true }, " sel"), kc({ t: "Retry uploads on flaky networks", tags: ["next"] }), kc({ t: "Show upload progress per photo" }, " done")].join(""), "␣ n t")}
       ${kcol("Linked here", 4, ["sync-design", "reading", "release-2.4", "journal · Oct 1"].map((n) => `<div class="kcard link"><span class="olink">${n}</span></div>`).join(""), "␣ n b")}</div>`, "split5"),
  },
  tasks: {
    1: () => shell1("tasks", head("Tasks", "All open tasks · 12", taskControls) + card("Next 7 days", agenda(4), { hint: "␣ a", count: 8 }) + `<div class="two">${unscheduled}${inbox}</div>`, { extra: hints }),
    2: () => shell2("tasks", `<div class="greet s12"><h1>Tasks</h1><span class="muted">12 open</span><span class="stat late">3 overdue</span><span class="stat">2 doing</span><span class="grow"></span>${taskControls}</div>
      ${card("Next 7 days", agenda(5), { cls: "s7 r3", hint: "␣ a", count: 8 })}${inbox.replace('class="card ', 'class="card s5 ')}${byTag.replace('class="card ', 'class="card s5 ')}${`<div class="s5 two">${doneWeek}${views}</div>`}`),
    5: () => shell5("tasks", "All tasks", "Grouped by tag · drag or press a column key", chips(["All", "work", "home"]) + `<button class="primary">+ New task</button>`,
      `<div class="lanes">${["Todo", "Next", "Doing", "Done"].map((c, i) => `<h2 class="lh ${["", "", "doing", ""][i]}">${c}<span class="count">${board[c].length}</span><span class="hint">${kbd("tngx"[i])}</span></h2>`).join("")}
       ${["work", "home"].map((tag) => `<div class="lane-h"><span class="tag">${tag}</span><span class="muted small">${Object.values(board).flat().filter((r) => r.tags?.includes(tag)).length} tasks</span></div>` +
        ["Todo", "Next", "Doing", "Done"].map((c) => `<div class="kcol cell ${c === "Doing" ? "doing" : ""}">${board[c].filter((r) => (r.tags ?? ["home"]).includes(tag)).map((r, i) => kc(r, c === "Done" ? " done" : c === "Doing" && !i && tag === "work" ? " sel" : "")).join("")}</div>`).join("")).join("")}</div>`, "lanes5"),
    mix: () => `<div class="b1 mix">${rail4("tasks")}<main><div class="subhead">${head("Tasks", "All open tasks · board", chips(["All", "work", "home"]) + seg(["List", "Board"], 1) + `<button class="primary">+ New task</button>`)}</div>
      <div class="board v-today">${boardCols()}</div>${boardHints}</main></div>`,
  },
  projects: {
    1: () => shell1("projects", head("Projects", "3 active · 1 hidden", `<button>Show hidden</button><button>${kbd("␣ p o")}Time off</button>`) + `<div class="two">${P4.map((p) => pcard(p)).join("")}${hiddenCard}</div>${timeOff}`),
    2: () => shell2("projects", `<div class="greet s12"><h1>Projects</h1><span class="muted">week 40</span><span class="stat">35h 55m tracked</span><span class="stat">36.0 h billable</span></div>
      ${P4.map((p) => pcard(p, "s4")).join("")}${allProjects.replace('class="card ', 'class="card s8 ')}${timeOff.replace('class="card ', 'class="card s4 ')}`),
    5: () => shell5("projects", "Projects", "Open tasks by project · week 40", `<button>Show hidden</button>`,
      P4.map(([n, c, w, , s], i) => kcol(n, w, `<div class="ptot"><div class="track"><i style="width:${s * 2}%;background:${pc[n]}"></i></div><span class="muted small mono">${c} · ${projs[i][3]} billable</span></div>` +
        Object.values(board).flat().filter((r) => r.file === n.toLowerCase()).slice(0, 4).map((r) => kc({ ...r, file: "" }, board.Done.includes(r) ? " done" : "")).join("") +
        (n === "Internal" ? kc({ t: "Team sync notes", time: "Mon 10:00" }) : ""), String(i + 1))).join("") +
      `<section class="kcol hiddencol"><h2>Hidden<span class="count">1</span></h2><div class="kcard"><div class="kt"><i class="sw" style="background:${pc.Garden}"></i>Garden</div><div class="kmeta"><span class="muted">2h 00m · Sep</span></div></div>${timeOff}</section>`, "board pcols"),
  },
  settings: {
    1: () => shell1("settings", setHead() + chips(Object.keys(SET)) + setCard("Folders") + `<div class="two">${setCard("Time")}${setCard("Tasks")}</div>` + setCard("Reminders & desktop")),
    2: () => shell2("settings", `<div class="greet s12">${setHead("config.toml · saved as you type · 5 groups")}</div>${setCard("Folders", "s6")}${setCard("Reminders & desktop", "s6")}${setCard("Time", "s4")}${setCard("Tasks", "s4")}${setCard("Integrations", "s4")}`),
    5: () => shell5("settings", "Settings", "config.toml · saved as you type", `<span class="filter">Filter ${kbd("/")}</span>`,
      `<section class="kcol groups"><h2>Groups</h2>${Object.keys(SET).map((g, i) => `<div class="kcard${i ? "" : " sel"}"><div class="kt">${g}<span class="count">${SET[g].length}</span></div></div>`).join("")}<button class="add">Edit config.toml</button></section>
       <div class="fcards">${["Folders", "Time", "Reminders & desktop"].map((g) => `<h3 class="fgh">${g}</h3>` + SET[g].map(([l, c, h]) => `<div class="kcard field"><div><span>${l}</span>${h ? `<small>${h}</small>` : ""}</div>${c}</div>`).join("")).join("")}</div>`, "set5"),
  },
  palette: {
    1: () => shell1("today", today1(), { extra: todayHints() + palette() }),
    2: () => shell2("today", today2()).replace(/<\/div>$/, palette() + "</div>"),
    5: () => today5().replace(/<\/div>$/, palette() + "</div>"),
  },
  graph: {
    1: () => shell1("graph", head("Graph", "15 notes · 16 links", `<button>Depth 2 ▾</button>`) + twoCol(graphCard(), selNote + gfilters + orphans), { wide: true }),
    2: () => shell2("graph", `${graphCard("s8 r3")}${selNote.replace('class="card ', 'class="card s4 ')}${gfilters.replace('class="card ', 'class="card s4 ')}${orphans.replace('class="card ', 'class="card s4 ')}`),
    5: () => shell5("graph", "Graph", "15 notes · 16 links · harbor selected", `<button>Depth 2 ▾</button>`,
      `${graphCard()}<div class="kside">${kcol("Linked to harbor", 5, ["sync-design", "reading", "release-2.4", "orchard", "journal · Oct 1"].map((n) => `<div class="kcard link"><span class="olink">${n}</span><span class="muted small">${deg(n)} link${deg(n) > 1 ? "s" : ""}</span></div>`).join(""), "␣ n l")}</div>`, "split5"),
  },
  today: { 1: () => shell1("today", today1(), { extra: todayHints() }), 2: () => shell2("today", today2()), 5: today5 },
  time: { 1: () => shell1("time", view("time", () => frag(b1(), ".col").innerHTML)) },
};
document.documentElement.dataset.s = S;
if (!window.R5) document.body.innerHTML = DEMO ? "" : (PAGES[P][S] ?? PAGES[P][1])(); // r5.js reuses this file without rendering it

/* Motion demos. Each loops every LOOP ms with the same durations and easing as the CSS transitions in r4.css. */
const EASE = "cubic-bezier(.2, 0, 0, 1)";
let LOOP = 4000;
// The resolved value of the custom property N (var() and color-mix() resolved, so animations can interpolate it).
const css = (n, prop = /soft|lift|ring/.test(n) ? "boxShadow" : "color") => {
  const p = document.body.appendChild(document.createElement("i")); p.style[prop] = `var(${n})`;
  const r = getComputedStyle(p)[prop]; p.remove(); return r;
};
// Animate EL from INIT through STEPS ([at ms, duration ms, styles]); INIT must name every property used.
const seq = (el, init, steps) => {
  const kf = [{ offset: 0, ...init }]; let cur = init;
  for (const [at, d, s] of steps) { kf.push({ offset: at / LOOP, ...cur }); cur = { ...cur, ...s }; kf.push({ offset: (at + d) / LOOP, ...cur }); }
  kf.push({ offset: 1, ...cur });
  return el.animate(kf.map((k) => ({ easing: EASE, ...k })), { duration: LOOP, iterations: Infinity });
};
const keycap = (k, at, hold = 700) => {
  document.body.insertAdjacentHTML("beforeend", `<div class="keytoast">${k.split(" ").map(kbd).join("")}</div>`);
  seq(document.body.lastElementChild, { opacity: 0, transform: "translate(-50%, 4px)" }, [[at, 140, { opacity: 1, transform: "translate(-50%, 0)" }], [at + hold, 160, { opacity: 0 }]]);
};
const rect = (el) => el.getBoundingClientRect();
const DEMOS = {
  // View change in B1: Today → Tasks → Time → Today, the rail highlight slides along.
  view() {
    LOOP = 4800;
    const ids = ["today", "tasks", "time"], bodies = [today1(), PAGES.tasks[1]().match(/<div class="col">([\s\S]*)<\/div><footer/)[1], view("time", () => frag(b1(), ".col").innerHTML)];
    document.body.innerHTML = `<div class="b1 demo">${rail4("")}<main class="layers">${bodies.map((b, i) => `<div class="layer" data-v="${ids[i]}"><div class="col">${b}</div></div>`).join("")}</main></div>`;
    const railEl = document.querySelector(".rail"), items = Object.fromEntries(ids.map((id) => [id, railEl.querySelector(`[data-p="${id}"]`)]));
    railEl.insertAdjacentHTML("afterbegin", `<span class="railind"></span>`);
    const ind = railEl.firstElementChild, y = (id) => `translateY(${items[id].offsetTop}px)`;
    ind.style.height = items.today.offsetHeight + "px";
    const plan = [[1200, "tasks", "Ctrl 2"], [2600, "time", "Ctrl 4"], [4000, "today", "Ctrl 1"]];
    seq(ind, { transform: y("today") }, plan.map(([at, id]) => [at, 180, { transform: y(id) }]));
    const fg = css("--fg"), dim = css("--dim");
    for (const id of ids) seq(items[id], { color: id === "today" ? fg : dim }, plan.map(([at, to]) => [at, 180, { color: to === id ? fg : dim }]));
    const layers = [...document.querySelectorAll(".layer")];
    for (const l of layers) {
      const id = l.dataset.v, steps = [];
      let shown = id === "today";
      for (const [at, to] of plan) {
        if (shown && to !== id) { steps.push([at, 120, { opacity: 0 }]); shown = false; }
        else if (!shown && to === id) { steps.push([at + 59, 1, { transform: "translateY(4px)" }], [at + 60, 180, { opacity: 1, transform: "translateY(0px)" }]); shown = true; }
      }
      seq(l, { opacity: id === "today" ? 1 : 0, transform: "translateY(0px)" }, steps);
    }
    plan.forEach(([at, , k]) => keycap(k, at - 150, 500));
  },
  // Card hover lift and keyboard focus ring in B2's dashboard.
  hover() {
    LOOP = 4800;
    document.body.innerHTML = shell2("today", today2()) + `<svg class="fakecursor" viewBox="0 0 16 22"><path d="M1 1v17l4.5-4 3 7 3-1.4-3-6.6H15z"/></svg>`;
    const [sch, run, , jr] = document.querySelectorAll(".grid > .card");
    const soft = css("--soft"), lift = css("--lift"), ring = css("--ring");
    const none = `0 0 0 0px transparent`, at = (el, fx = 0.5, fy = 0.5) => { const r = rect(el); return `translate(${r.left + r.width * fx}px, ${r.top + r.height * fy}px)`; };
    const rows = sch.querySelectorAll(".task");
    const cur = document.querySelector(".fakecursor"), home = "translate(1180px, 720px)";
    seq(cur, { transform: home }, [[300, 500, { transform: at(rows[1], 0.35) }], [1400, 300, { transform: at(rows[2], 0.4) }], [2100, 500, { transform: at(run, 0.4, 0.45) }], [3100, 500, { transform: home }]]);
    const lifted = (el, steps) => seq(el, { transform: "translateY(0px)", boxShadow: `${soft}, ${none}` }, steps);
    const up = { transform: "translateY(-2px)", boxShadow: `${lift}, ${none}` }, down = { transform: "translateY(0px)", boxShadow: `${soft}, ${none}` };
    lifted(sch, [[800, 140, up], [2200, 140, down]]);
    lifted(run, [[2600, 140, up], [3200, 140, down]]);
    lifted(jr, [[3600, 140, { boxShadow: `${soft}, ${ring}` }], [4400, 140, { boxShadow: `${soft}, ${none}` }]]);
    const bg = css("--active");
    rows.forEach((r) => r.classList.remove("sel"));
    seq(rows[1], { backgroundColor: "transparent" }, [[800, 120, { backgroundColor: bg }], [1600, 120, { backgroundColor: "transparent" }]]);
    seq(rows[2], { backgroundColor: "transparent" }, [[1600, 120, { backgroundColor: bg }], [2200, 120, { backgroundColor: "transparent" }]]);
    keycap("Tab", 3450, 600);
  },
  // Palette: Ctrl K opens it, typing filters, ↓↑ move the selection, ↵ runs "Go to Time tracking".
  palette() {
    LOOP = 6400;
    const ids = ["today", "time"];
    const bodies = [today1(), view("time", () => frag(b1(), ".col").innerHTML)];
    document.body.innerHTML = `<div class="b1 demo">${rail4("")}<main class="layers">${bodies.map((b, i) => `<div class="layer" data-v="${ids[i]}"><div class="col">${b}</div></div>`).join("")}</main></div>${palette("time")}`;
    const railEl = document.querySelector(".rail"), it = (id) => railEl.querySelector(`[data-p="${id}"]`);
    railEl.insertAdjacentHTML("afterbegin", `<span class="railind"></span>`);
    const ind = railEl.firstElementChild; ind.style.height = it("today").offsetHeight + "px";
    const fg = css("--fg"), dim = css("--dim");
    seq(ind, { transform: `translateY(${it("today").offsetTop}px)` }, [[3500, 180, { transform: `translateY(${it("time").offsetTop}px)` }], [4900, 180, { transform: `translateY(${it("today").offsetTop}px)` }]]);
    seq(it("today"), { color: fg }, [[3500, 180, { color: dim }], [4900, 180, { color: fg }]]);
    seq(it("time"), { color: dim }, [[3500, 180, { color: fg }], [4900, 180, { color: dim }]]);
    const [lt, lm] = document.querySelectorAll(".layer");
    seq(lt, { opacity: 1, transform: "translateY(0px)" }, [[3500, 120, { opacity: 0 }], [4959, 1, { transform: "translateY(4px)" }], [4960, 180, { opacity: 1, transform: "translateY(0px)" }]]);
    seq(lm, { opacity: 0, transform: "translateY(4px)" }, [[3560, 180, { opacity: 1, transform: "translateY(0px)" }], [4900, 120, { opacity: 0 }], [5100, 1, { transform: "translateY(4px)" }]]);
    const back = document.querySelector(".pal-back"), pal = document.querySelector(".pal");
    seq(back, { opacity: 0 }, [[500, 160, { opacity: 1 }], [3400, 160, { opacity: 0 }]]);
    seq(pal, { opacity: 0, transform: "translate(-50%, -4px) scale(.98)" }, [[500, 160, { opacity: 1, transform: "translate(-50%, 0px) scale(1)" }], [3400, 120, { opacity: 0, transform: "translate(-50%, -4px) scale(.98)" }]]);
    document.querySelectorAll(".pal .ch").forEach((c, i) => seq(c, { fontSize: "0px" }, [[1000 + i * 150, 0, { fontSize: "15px" }], [3700, 0, { fontSize: "0px" }]]));
    seq(pal.querySelector(".ph"), { opacity: 1 }, [[1000, 0, { opacity: 0 }], [3700, 0, { opacity: 1 }]]);
    const la = pal.querySelector(".plist.a"), lb = pal.querySelector(".plist.b");
    seq(la, { opacity: 1 }, [[1150, 120, { opacity: 0 }], [3700, 0, { opacity: 1 }]]);
    seq(lb, { opacity: 0 }, [[1150, 120, { opacity: 1 }], [3700, 0, { opacity: 0 }]]);
    const li = (n) => lb.querySelectorAll("li")[n], sel = pal.querySelector(".palsel"), top = (n) => `translateY(${li(n).offsetTop + lb.offsetTop}px)`;
    lb.querySelectorAll("li.sel").forEach((x) => x.classList.remove("sel"));
    la.querySelectorAll("li.sel").forEach((x) => x.classList.remove("sel"));
    sel.style.height = li(0).offsetHeight + "px";
    seq(sel, { transform: `translateY(${la.querySelector("li").offsetTop + la.offsetTop}px)` }, [[1150, 120, { transform: top(0) }], [2100, 120, { transform: top(1) }], [2600, 120, { transform: top(0) }], [3700, 0, { transform: `translateY(${la.querySelector("li").offsetTop + la.offsetTop}px)` }]]);
    keycap("Ctrl K", 300, 500); keycap("↓", 2000, 300); keycap("↑", 2500, 300); keycap("↵", 3200, 400); keycap("Ctrl 1", 4700, 500);
  },
  // A task moves from Doing to Done with x, and back with g (on B5's Today board, or the board r5.js passes in).
  move(html = today5()) {
    LOOP = 4800;
    document.body.innerHTML = html;
    document.documentElement.classList.add("demo-move");
    const cols = [...document.querySelectorAll(".kcol")], doing = cols[2], donec = cols[3];
    const card = doing.querySelectorAll(".kcard")[1], firstDone = donec.querySelector(".kcard");
    const gap = parseFloat(getComputedStyle(doing).rowGap) || 8, h = rect(card).height + gap;
    const dx = rect(firstDone).left - rect(card).left, dy = rect(firstDone).top - rect(card).top;
    const soft = css("--soft"), lift = css("--lift"), ring = css("--ring"), none = "0 0 0 0px transparent";
    doing.querySelector(".kcard.sel")?.classList.remove("sel");
    const home = "translate(0px, 0px) scale(1)", there = `translate(${dx}px, ${dy}px) scale(1)`;
    seq(card, { transform: home, boxShadow: `${soft}, ${ring}` }, [
      [500, 120, { transform: "translate(0px, 0px) scale(1.02)", boxShadow: `${lift}, ${ring}` }], [620, 200, { transform: `translate(${dx}px, ${dy}px) scale(1.02)` }], [820, 120, { transform: there, boxShadow: `${soft}, ${ring}` }],
      [2800, 120, { transform: `translate(${dx}px, ${dy}px) scale(1.02)`, boxShadow: `${lift}, ${ring}` }], [2920, 200, { transform: "translate(0px, 0px) scale(1.02)" }], [3120, 120, { transform: home, boxShadow: `${soft}, ${ring}` }]]);
    seq(card.querySelector(".kt"), { textDecorationLine: "none", color: css("--fg") }, [[820, 120, { textDecorationLine: "line-through", color: css("--dim") }], [2920, 120, { textDecorationLine: "none", color: css("--fg") }]]);
    for (const el of [...donec.querySelectorAll(".kcard, .add")]) seq(el, { transform: "translateY(0px)" }, [[620, 200, { transform: `translateY(${h}px)` }], [2920, 200, { transform: "translateY(0px)" }]]);
    seq(doing.querySelector(".add"), { transform: "translateY(0px)" }, [[620, 200, { transform: `translateY(${-h}px)` }], [2920, 200, { transform: "translateY(0px)" }]]);
    for (const [col, a, z] of [[doing, 2, 1], [donec, 3, 4]]) {
      const c = col.querySelector(".count"); c.innerHTML = `<span>${a}</span><span>${z}</span>`; c.classList.add("swap");
      seq(c.children[0], { opacity: 1 }, [[720, 120, { opacity: 0 }], [3020, 120, { opacity: 1 }]]);
      seq(c.children[1], { opacity: 0 }, [[720, 120, { opacity: 1 }], [3020, 120, { opacity: 0 }]]);
    }
    keycap("x", 300, 500); keycap("g", 2600, 500);
  },
  // The running clock ticks: each minute rolls in 160 ms; the dot breathes (CSS).
  clock() {
    LOOP = 4800;
    document.body.innerHTML = shell1("time", view("time", () => frag(b1(), ".col").innerHTML));
    const roll = (el, from) => {
      const ds = [0, 1, 2, 3, 0].map((k) => (from + k) % 10), base = el.textContent.replace(/(\d)m$/, "");
      el.innerHTML = `${base}<span class="roll"><span class="reel">${ds.map((d) => `<span>${d}</span>`).join("")}</span></span>m`;
      seq(el.querySelector(".reel"), { transform: "translateY(0em)" }, [800, 2000, 3200, 4400].map((at, i) => [at, 160, { transform: `translateY(${-1.25 * (i + 1)}em)` }]));
    };
    document.querySelectorAll(".running .total b, .rail .timer .lbl, .live td.r").forEach((el) => roll(el, +el.textContent.match(/(\d)m$/)[1]));
  },
};
if (window.R5) { /* r5.js runs its own demos */ }
else if (DEMO && matchMedia("(prefers-reduced-motion: reduce)").matches) // no motion: the demo's still page
  document.body.innerHTML = { view: PAGES.today[1], hover: PAGES.today[2], palette: PAGES.palette[1], move: PAGES.today[5], clock: PAGES.time[1] }[DEMO]();
else if (DEMO) { document.documentElement.classList.add("demo"); DEMOS[DEMO](); }
if (T != null && !window.R5) document.getAnimations().forEach((a) => { a.pause(); a.currentTime = +T; });
