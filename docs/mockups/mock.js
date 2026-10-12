// Renders the Today or Time view with the demo data of e2e/screenshots.ts; the
// direction stylesheet (a-quiet / b-cards / c-dense .css) does all the styling.
const q = new URLSearchParams(location.search);
const d = q.get("d") ?? "a", v = q.get("v") ?? "today";
const css = { a: "a-quiet", b: "b-cards", c: "c-dense" }[d];
document.head.insertAdjacentHTML("beforeend", `<link rel="stylesheet" href="${css}.css">`);
document.documentElement.dataset.d = d;

const tags = (t) => t.map((x) => `<span class="tag">${x}</span>`).join("");
const kbd = (k) => `<kbd>${k}</kbd>`;

// [state, priority, title, tags, meta (class, text), file]
const today = [
  ["", "", "Review the release checklist", ["work"], ["time", "14:00"], "harbor"],
  "now",
  ["", "", "Book a dentist appointment", ["home"], null, "home"],
  ["", "", "Stretch for ten minutes", [], ["habit", ""], "home"],
  ["NEXT", "", "Write the partner onboarding guide", ["work", "docs"], null, "orchard"],
];
const late = [
  ["", "B", "Fix the crash when a photo upload is cancelled", ["work", "bug"], ["late", "Scheduled 1d ago"], "harbor"],
  ["", "A", "Renew the bike insurance", ["home"], ["late", "Due in 5d"], "home"],
  ["", "A", "Load test the rate limiter", ["work"], ["late", "Due in 2d"], "orchard"],
];
const habit = "111110111111101111".split("").map((c) => `<i class="${c === "1" ? "y" : "n"}"></i>`).join("");
const row = (r, i) => r === "now"
  ? `<li class="now"><span>now 15:58</span></li>`
  : `<li class="task${i === 0 ? " sel" : ""}"><span class="box"></span>
      ${r[0] ? `<span class="kw">${r[0]}</span>` : ""}${r[1] ? `<span class="pri p${r[1]}">${r[1]}</span>` : ""}
      <span class="title">${r[2]}</span>${tags(r[3])}
      <span class="meta">${r[4]?.[0] === "habit" ? `<span class="habit">${habit}</span><span class="streak">3 in a row</span>` : r[4] ? `<span class="${r[4][0]}">${r[4][1]}</span>` : ""}<span class="file">${r[5]}</span></span></li>`;

const nav = [["◎", "Today", "1"], ["☑", "Tasks", "2"], ["↓", "Inbox", "3"], ["✎", "Journal", "J"], ["◷", "Time", "4"], ["⌕", "Work this week", ""]];
const side = `<aside class="side">
  <button class="cmdk"><span>⌕</span><span class="lbl">Search or run a command…</span>${kbd("Ctrl K")}</button>
  <nav>${nav.map(([i, l, k]) => `<a class="${(v === "time" ? "Time" : "Today") === l ? "on" : ""}"><span class="ico">${i}</span><span class="lbl">${l}</span>${k ? kbd("Ctrl " + k) : ""}</a>`).join("")}</nav>
  <div class="sec">Notes</div>
  <ul class="notes">${["orchard", "harbor", "sync-design", "reading"].map((n) => `<li>${n}</li>`).join("")}</ul>
  <div class="timer"><div class="what"><span class="dot"></span><b>Harbor</b></div><small>5h 35m today</small>
    <div class="tbtns"><button>Pause</button><button>Stop</button></div></div>
</aside>`;
const tabs = `<div class="tabs">${["Today", "reading", "sync-design", "harbor", "orchard", ...(v === "time" ? ["Time"] : [])]
  .map((t, i, a) => `<span class="${i === (v === "time" ? a.length - 1 : 0) ? "on" : ""}">${t}</span>`).join("")}</div>`;

const todayView = `<section class="view today">
  <header><div><h1>Today</h1><div class="sub">Friday, October 2</div></div>
    <div class="controls"><span class="seg"><b>Day</b><span>Week</span></span><button>‹</button><button>Today</button><button>›</button>
      <span class="filter">Filter ${kbd("/")}</span><button class="primary">+ New task</button></div></header>
  <div class="group"><h2>Scheduled <span class="count">4</span></h2><ul>${today.map(row).join("")}</ul></div>
  <div class="group"><h2>Overdue and deadlines <span class="count">3</span></h2><ul>${late.map((r) => row(r, 1)).join("")}</ul></div>
</section>
<footer class="hints">${[["x", "Done"], ["s", "Schedule"], ["d", "Due date"], ["p", "Priority"], ["t", "Status"], ["#", "Tags"], ["m", "Move to…"], ["I", "Track time"], ["↵", "Open"]]
  .map(([k, l]) => `<span>${kbd(k)}${l}</span>`).join("")}<span class="more">${kbd("?")}More</span></footer>`;

const week = [["Mon", "8h 05m", 97], ["Tue", "8h 15m", 100], ["Wed", "8h 05m", 97], ["Thu", "8h 15m", 100], ["Fri", "5h 36m", 68], ["Sat", "–", 0], ["Sun", "–", 0]];
const timeView = `<section class="view time">
  <header><div><h1>Time</h1><div class="sub">Week 40 · 38h 16m of 40h</div></div>
    <div class="controls"><span class="muted">Profile</span><button class="select">Work ▾</button></div></header>
  <div class="running">
    <div class="who"><span class="dot"></span><div><small>Tracking since 13:38</small><h3>Harbor</h3><p>Fix the crash when a photo upload is cancelled</p></div></div>
    <div class="total"><b>5h 36m</b><small>today</small></div>
    <div class="acts"><button>${kbd("p")}Pause</button><button>${kbd("c")}Switch project</button><button>${kbd("a")}Started earlier…</button><button class="primary">${kbd("o")}Stop</button></div>
  </div>
  <div class="cols">
    <div class="panel sessions"><h2>Today</h2><table>
      <tr><td class="mono">09:28–12:13</td><td><b>Orchard</b> <span class="muted">Load test plan</span></td><td class="mono r">2h 45m</td></tr>
      <tr><td class="mono">12:58–13:28</td><td><b>Internal</b> <span class="muted">Team sync</span></td><td class="mono r">30m</td></tr>
      <tr class="live"><td class="mono">13:38–now</td><td><b>Harbor</b> <span class="muted">Photo upload fixes</span></td><td class="mono r">2h 20m</td></tr></table></div>
    <div class="panel week"><h2>This week <span class="muted">38h 16m of 40h 00m</span></h2>
      <div class="bars">${week.map(([dn, h, p]) => `<div class="day${dn === "Fri" ? " cur" : ""}"><div class="bar"><i style="height:${p}%"></i></div><span class="h">${h}</span><span class="dn">${dn}</span></div>`).join("")}</div>
      <div class="flex"><div><small>Flex balance</small><b class="neg">-3h 25m</b></div><div><small>This week</small><b class="neg">-4h 05m</b></div></div></div>
  </div>
  <div class="panel projects"><h2>Projects this week</h2><table>
    <tr><th>Project</th><th>Code</th><th class="r">Worked</th><th class="r">Billable</th></tr>
    ${[["Orchard", "ORC-100", "17h 45m", "17.75 h", 47], ["Harbor", "HRB-210", "13h 30m", "13.50 h", 35], ["Internal", "INT", "4h 40m", "4.67 h", 12]]
      .map(([p, c, w, b, s]) => `<tr><td>${p}<span class="share" style="width:${s}%"></span></td><td class="mono muted">${c}</td><td class="mono r">${w}</td><td class="mono r">${b}</td></tr>`).join("")}</table></div>
  <div class="more-acts">${[["t", "Day report"], ["w", "Week report"], ["e", "Export CSV…"], ["E", "Export report…"]].map(([k, l]) => `<button>${kbd(k)}${l}</button>`).join("")}<button>Project settings…</button></div>
</section>`;

document.body.innerHTML = `<div class="app">${side}<main>${tabs}${v === "time" ? timeView : todayView}<div class="status">Space menu · Ctrl+K commands</div></main></div>`;
