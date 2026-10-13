// Round 5 of #251, the combined draft: B1's top bar (its rail laid out sideways), B5's layout (title row with the
// agenda chips, then full-width columns) and B2's cards inside the columns. Reuses b-variants.js and r4.js (both
// loaded first and told not to render themselves). ?p=…, &bar=rail, ?demo=view|palette|move, &t=ms as in round 4.
const RAIL = q.get("bar") === "rail";

/* Shell */
const bar = (p) => `<header class="topnav"><span class="logo">m</span><nav class="views">${NAV.map(([i, l, k, id]) =>
  `<a class="${p === id ? "on" : ""}" data-p="${id}"><span class="ico">${i}</span><span class="lbl">${l}</span>${p === id ? "" : kbd(k)}</a>`).join("")}</nav>
  <span class="grow"></span>${search}<a class="set${p === "settings" ? " on" : ""}"><span class="ico">⚙</span></a>${chip}</header>`;
const agChips = `<div class="agenda">${[["09:28", "Orchard · load test plan", "done"], ["13:38", "Harbor · tracking", "live"], ["14:00", "Release checklist", ""], ["16:30", "Dentist call", ""]]
  .map(([t, l, c]) => `<span class="ag ${c}"><b class="mono">${t}</b>${l}</span>`).join("")}</div>`;
// A B5 column; NAME "" leaves out its header (the project columns are headed by their B2 card).
const col = (name, cards, { n = "", key = "", cls = "" } = {}) => `<section class="kcol ${cls}">${name ? `<h2>${name}${n !== "" ? `<span class="count">${n}</span>` : ""}${key ? `<span class="hint">${kbd(key)}</span>` : ""}</h2>` : ""}${cards}</section>`;
const page = (p, title, sub, controls, cols, { tmpl, top = "", extra = "" }) => {
  const work = `<div class="work"><div class="subhead">${head(title, sub, controls)}${top}</div><main class="cols" style="grid-template-columns:${tmpl}">${cols}</main>${extra}</div>`;
  return RAIL ? `<div class="c5 railv">${rail4(p)}${work}</div>` : `<div class="c5">${bar(p)}${work}</div>`;
};

/* Pages */
const worked = card("Worked today", `<div class="big mono">5h 36m</div><div class="progress"><i style="width:70%"></i></div><span class="muted small">of 8h 00m · 2h 24m to go</span>`, { hint: "␣ t" });
const P5 = {
  today: () => page("today", "Today", "Friday, October 2", dayControls,
    col("Clock", running(true) + worked + card("This week", bars(64), { hint: "4" }), { key: "I" }) +
    col("Agenda", card("Scheduled", list(sched, { sel: true, nowAt: 0 }), { count: 4, hint: "␣ a" }) + card("Overdue and deadlines", list(late, { compact: true }), { count: 3, hint: "␣ o" }), { n: 7, cls: "agcol" }) +
    col("Daily", journal + habits), { tmpl: "1fr 1.5fr 1fr", top: agChips, extra: hints }),
  time: () => page("time", "Time", "Week 40 · 38h 16m of 40h", profile,
    col("Clock", running() + worked + card("Balance", flex + reports, { hint: "b" }), { key: "I" }) +
    col("Sessions", sessions("␣ t") + card("This week", bars(110), { hint: "w" }), { n: 3 }) +
    col("Projects", P4.map((p) => pcard(p)).join(""), { n: 3, key: "5" }), { tmpl: "1fr 1fr 1fr", top: agChips }),
  tasks: () => page("tasks", "Tasks", "All open tasks · 12 · board", chips(["All", "work", "home", "docs"]) + seg(["List", "Board"], 1) + `<button class="primary">+ New task</button>`,
    boardCols() + col("Overview", doneWeek + byTag + views, { cls: "side" }), { tmpl: "repeat(4, 1fr) 1.15fr", extra: boardHints }),
  projects: () => page("projects", "Projects", "Week 40 · 35h 55m tracked · 36.0 h billable", `<button>Show hidden</button><button class="primary">+ New project</button>`,
    P4.map((p) => col("", pcard(p) + `<div class="ksub">Open tasks</div>` + Object.values(board).flat().filter((r) => r.file === p[0].toLowerCase()).slice(0, 3)
      .map((r) => kc({ ...r, file: "" }, board.Done.includes(r) ? " done" : "")).join("") + (p[0] === "Internal" ? kc({ t: "Team sync notes", time: "Mon 10:00" }) : ""), { cls: "pcol" })).join("") +
    col("Hidden and time off", hiddenCard + timeOff, { cls: "side" }), { tmpl: "repeat(3, 1fr) 1.1fr" }),
  settings: () => page("settings", "Settings", "config.toml · saved as you type", `<span class="filter">Filter settings ${kbd("/")}</span><button>Edit config.toml</button>`,
    col("Files", setCard("Folders") + setCard("Integrations")) + col("Work", setCard("Time") + setCard("Tasks")) + col("Desktop", setCard("Reminders & desktop")),
    { tmpl: "1fr 1fr 1fr", top: chips(["All", ...Object.keys(SET)]) }),
  editor: () => page("editor", "harbor.org", "Notes · edited 2 min ago", `<button>${kbd("␣ n b")}Links</button><button>${kbd("Ctrl P")}Open note…</button>`,
    editorCard() + col("This note", outline + backlinks + timeOn, { cls: "side" }), { tmpl: "1fr 320px" }),
  graph: () => page("graph", "Graph", "15 notes · 16 links · harbor selected", `<button>Depth 2 ▾</button>`,
    graphCard() + col("Selected", selNote + gfilters + orphans, { cls: "side" }), { tmpl: "1fr 320px" }),
};
P5.palette = () => P5.today() + palette();

/* Motion: the round-4 demos whose look changes with this shell. Hover and clock are as in round 4. */
// Stacks the pages IDS under one bar and returns go(plan, first): the bar highlight slides to each [at ms, id] of PLAN
// while the pages cross-fade (out 120 ms, in 180 ms with a 4 px rise), the same timing as round 4's rail.
const layered = (ids, overlay = "") => {
  document.body.innerHTML = `<div class="c5 demo">${bar("")}<div class="work layers">${ids.map((id) => `<div class="layer" data-v="${id}">${frag(P5[id](), ".work").innerHTML}</div>`).join("")}</div></div>${overlay}`;
  const nav = document.querySelector(".views"), it = (id) => nav.querySelector(`[data-p="${id}"]`);
  nav.insertAdjacentHTML("afterbegin", `<span class="barind"></span>`);
  const ind = nav.firstElementChild, x = (id) => ({ transform: `translateX(${it(id).offsetLeft}px)`, width: `${it(id).offsetWidth}px` });
  return (plan, first) => {
    seq(ind, x(first), plan.map(([at, id]) => [at, 180, x(id)]));
    const fg = css("--fg"), dim = css("--dim");
    for (const id of ids) seq(it(id), { color: id === first ? fg : dim }, plan.map(([at, to]) => [at, 180, { color: to === id ? fg : dim }]));
    for (const l of document.querySelectorAll(".layer")) {
      const id = l.dataset.v, steps = [];
      let shown = id === first;
      for (const [at, to] of plan) {
        if (shown && to !== id) { steps.push([at, 120, { opacity: 0 }]); shown = false; }
        else if (!shown && to === id) { steps.push([at + 59, 1, { transform: "translateY(4px)" }], [at + 60, 180, { opacity: 1, transform: "translateY(0px)" }]); shown = true; }
      }
      seq(l, { opacity: id === first ? 1 : 0, transform: "translateY(0px)" }, steps);
    }
  };
};
const DEMOS5 = {
  // View change: Today → Tasks → Time → Today with Ctrl 2 / 4 / 1.
  view() {
    LOOP = 4800;
    const plan = [[1200, "tasks", "Ctrl 2"], [2600, "time", "Ctrl 4"], [4000, "today", "Ctrl 1"]];
    layered(["today", "tasks", "time"])(plan, "today");
    plan.forEach(([at, , k]) => keycap(k, at - 150, 500));
  },
  // Palette: Ctrl K opens it, typing filters, ↓↑ move the selection, ↵ runs "Go to Time tracking"; Ctrl 1 goes back.
  palette() {
    LOOP = 6400;
    layered(["today", "time"], palette("time"))([[3500, "time"], [4900, "today"]], "today");
    const back = document.querySelector(".pal-back"), pal = document.querySelector(".pal");
    seq(back, { opacity: 0 }, [[500, 160, { opacity: 1 }], [3400, 160, { opacity: 0 }]]);
    seq(pal, { opacity: 0, transform: "translate(-50%, -4px) scale(.98)" }, [[500, 160, { opacity: 1, transform: "translate(-50%, 0px) scale(1)" }], [3400, 120, { opacity: 0, transform: "translate(-50%, -4px) scale(.98)" }]]);
    document.querySelectorAll(".pal .ch").forEach((c, i) => seq(c, { fontSize: "0px" }, [[1000 + i * 150, 0, { fontSize: "15px" }], [3700, 0, { fontSize: "0px" }]]));
    seq(pal.querySelector(".ph"), { opacity: 1 }, [[1000, 0, { opacity: 0 }], [3700, 0, { opacity: 1 }]]);
    const la = pal.querySelector(".plist.a"), lb = pal.querySelector(".plist.b");
    seq(la, { opacity: 1 }, [[1150, 120, { opacity: 0 }], [3700, 0, { opacity: 1 }]]);
    seq(lb, { opacity: 0 }, [[1150, 120, { opacity: 1 }], [3700, 0, { opacity: 0 }]]);
    pal.querySelectorAll("li.sel").forEach((x) => x.classList.remove("sel"));
    const top = (l, n) => `translateY(${l.querySelectorAll("li")[n].offsetTop + l.offsetTop}px)`, sel = pal.querySelector(".palsel");
    sel.style.height = lb.querySelector("li").offsetHeight + "px";
    seq(sel, { transform: top(la, 0) }, [[1150, 120, { transform: top(lb, 0) }], [2100, 120, { transform: top(lb, 1) }], [2600, 120, { transform: top(lb, 0) }], [3700, 0, { transform: top(la, 0) }]]);
    keycap("Ctrl K", 300, 500); keycap("↓", 2000, 300); keycap("↑", 2500, 300); keycap("↵", 3200, 400); keycap("Ctrl 1", 4700, 500);
  },
  // A task moves Doing → Done with x and back with g, on the Tasks board (round 4's demo on this page).
  move() { DEMOS.move(P5.tasks()); },
};

if (DEMO && matchMedia("(prefers-reduced-motion: reduce)").matches) document.body.innerHTML = { view: P5.today, palette: P5.palette, move: P5.tasks }[DEMO]();
else if (DEMO) { document.documentElement.classList.add("demo"); DEMOS5[DEMO](); }
else if (q.has("w")) // &w=420: the page in a frame that wide (headless Chrome will not make its window that narrow)
  document.body.innerHTML = `<iframe src="${location.href.replace(/[?&]w=\d+/, "")}" style="width:${q.get("w")}px;height:100vh;border:0;display:block"></iframe>`;
else document.body.innerHTML = (P5[q.get("p")] ?? P5.today)();
if (T != null) document.getAnimations().forEach((a) => { a.pause(); a.currentTime = +T; });
