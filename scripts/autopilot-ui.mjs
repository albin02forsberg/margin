#!/usr/bin/env node
// Live dashboard for scripts/autopilot-forever.sh: reads `claude --output-format stream-json`
// on stdin and shows what is happening now, usage limits, context and what comes next.
// Plain timestamped lines when stdout is not a TTY (only if AUTOPILOT_LOG is unset, so lines aren't written twice).
// Env: AUTOPILOT_LOG (append plain lines), AUTOPILOT_LIMITS (write last rate-limit info as JSON),
//      AUTOPILOT_MAX (agent slots, default 3), AUTOPILOT_CONTEXT (context window tokens, default 200000),
//      AUTOPILOT_REPO (default albin02forsberg/margin).
import { createInterface } from "node:readline";
import { appendFileSync, writeFileSync, readFileSync, openSync, mkdirSync, existsSync } from "node:fs";
import { ReadStream } from "node:tty";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { execFile } from "node:child_process";
import { basename } from "node:path";
import { cut, readyRow, createArgs, issueNo, columns, budget, clip } from "./autopilot-ui-lib.mjs";

const tty = process.stdout.isTTY;
const logFile = process.env.AUTOPILOT_LOG;
const limitsFile = process.env.AUTOPILOT_LIMITS;
const MAX = Number(process.env.AUTOPILOT_MAX) || 3;
const CTX = Number(process.env.AUTOPILOT_CONTEXT) || 200000;
const REPO = process.env.AUTOPILOT_REPO || "albin02forsberg/margin";
const HEARTBEAT = process.env.AUTOPILOT_HEARTBEAT;   // touched on every event; the runner's stall watchdog reads it
const TAIL = process.env.AUTOPILOT_TAIL;             // last raw lines, written at the end (limit/auth detection)
const ROTATE = process.env.AUTOPILOT_ROTATE;         // marker written when the run is ended for context rotation
const SESSION = process.env.AUTOPILOT_SESSION;       // session id of the claude process, to stop it
const sgr = (n) => (s) => (tty ? `\x1b[${n}m${s}\x1b[0m` : s);
const [bold, dim, red, green, yellow, cyan] = [1, 2, 31, 32, 33, 36].map(sgr);

const started = Date.now();
const st = {
  tools: 0, agents: 0, issues: 0, prs: 0, merges: 0, commits: 0, model: "", now: "starting…", recent: [],
  ctx: 0, compactions: 0, limits: null, up: null, rotating: false,
  needs: [], panel: false, sel: 0, msel: 0, help: false, input: null, confirm: null, msg: "",
};
const OWNER = "albin02forsberg";
const STATE_DIR = limitsFile ? dirname(limitsFile) : join(homedir(), ".cache", "margin-autopilot");
const DISMISSED = join(STATE_DIR, "dismissed.json");
const standalone = process.argv.includes("--decide"); // decision panel only, no run attached
// --sleep <until-epoch-s> <why>: the runner is waiting for a reset; keep the dashboard up until then (or STOP).
const si = process.argv.indexOf("--sleep");
const sleepUntil = si > 0 ? Number(process.argv[si + 1]) * 1000 : 0, sleepWhy = si > 0 ? process.argv[si + 2] || "" : "";
let dismissed = new Set();
try { dismissed = new Set(JSON.parse(readFileSync(DISMISSED, "utf8"))); } catch {}
const notified = new Set();
const rawTail = [];
let lastBeat = 0;
const agents = new Map(); // Agent tool_use id -> { name, born, seen, last }

const clock = (t = Date.now()) => new Date(t).toTimeString().slice(0, 8);
const dur = (ms) => {
  const s = Math.max(0, Math.floor(ms / 1000)), d = Math.floor(s / 86400), h = Math.floor((s % 86400) / 3600), m = Math.floor((s % 3600) / 60);
  return d ? `${d}d${h}h` : h ? `${h}h${String(m).padStart(2, "0")}m` : `${m}m${String(s % 60).padStart(2, "0")}s`;
};
const one = (s) => String(s ?? "").replace(/\s+/g, " ").trim();
const tok = (n) => (n >= 1000 ? `${Math.round(n / 1000)}k` : String(n));
const bar = (f, w = 20) => {  // w cells wide
  const x = Math.max(0, Math.min(1, f)), n = Math.round(x * w);
  const col = x >= 0.9 ? red : x >= 0.7 ? yellow : green;
  return col("█".repeat(n)) + dim("░".repeat(w - n));
};

function describe(b) {
  const i = b.input ?? {};
  const arg = /^(Read|Edit|Write)$/.test(b.name) ? basename(i.file_path ?? "") : i.description ?? i.command ?? i.pattern ?? i.prompt ?? i.url ?? "";
  return `${b.name} ${one(arg)}`.trim();
}

function count(b) {
  st.tools++;
  const cmd = b.input?.command ?? "";
  if (b.name === "Agent") st.agents++;
  if (/gh issue create/.test(cmd)) st.issues++;
  if (/gh pr create/.test(cmd)) st.prs++;
  if (/gh pr merge/.test(cmd)) st.merges++;
  if (/git commit/.test(cmd)) st.commits++;
}

function note(text, parent, kind) {
  const line = one(text);
  if (!line) return;
  const a = parent && agents.get(parent);
  if (a) { a.seen = Date.now(); a.last = line; } else st.now = line;
  st.recent.push({ t: Date.now(), who: a?.name, kind, line });
  if (st.recent.length > 40) st.recent.shift();
  if (logFile) { try { appendFileSync(logFile, `${clock()} ${a ? `[${a.name}] ` : ""}${line}\n`); } catch {} }
  if (!tty && !logFile) console.log(`${clock()} ${a ? `  ↳ ${a.name}: ` : ""}${line}`);
}

function onEvent(e) {
  if (e.type === "system") {
    if (e.model) st.model = e.model;
    if (/compact/i.test(`${e.subtype} ${e.status_category} ${e.status_detail}`)) st.compactions++;
  } else if (e.type === "rate_limit_event" && e.rate_limit_info) {
    st.limits = { ...e.rate_limit_info, seen: Date.now() };
    if (limitsFile) { try { writeFileSync(limitsFile, JSON.stringify({ ts: Date.now(), ...e.rate_limit_info })); } catch {} }
    if (!tty && e.rate_limit_info.status !== "allowed") console.log(`${clock()} rate limit: ${e.rate_limit_info.status} (${e.rate_limit_info.rateLimitType})`);
  } else if (e.type === "assistant") {
    const u = e.message?.usage;
    if (u && !e.parent_tool_use_id) { // main (coordinator) context = everything sent in the last request
      const c = (u.input_tokens ?? 0) + (u.cache_creation_input_tokens ?? 0) + (u.cache_read_input_tokens ?? 0);
      if (st.ctx > 20000 && c < st.ctx * 0.6) st.compactions++;
      st.ctx = c;
    }
    for (const b of e.message?.content ?? []) {
      if (b.type === "text") note(b.text.slice(0, 300), e.parent_tool_use_id, "text");
      else if (b.type === "tool_use") {
        count(b);
        if (b.name === "Agent") {
          const name = one(b.input?.description ?? b.input?.subagent_type ?? "agent");
          agents.set(b.id, { name, born: Date.now(), seen: Date.now(), last: "starting…" });
        }
        note(describe(b), e.parent_tool_use_id, "tool");
      }
    }
  } else if (e.type === "result") {
    st.now = `run finished: ${one(e.result ?? e.subtype).slice(0, 200)}`;
    note(st.now, null, "done");
  }
}

// ---- what's coming up: cheap `gh` polling, no model tokens -------------------------------------
const gh = (args) => new Promise((res, rej) => execFile("gh", args, { maxBuffer: 20e6 }, (e, out) => (e ? rej(e) : res(out))));
const THREE_H = 3 * 3600e3;
async function pollUp() {
  try {
    const [iss, prs, runsJson] = await Promise.all([
      gh(["issue", "list", "-R", REPO, "--state", "open", "--limit", "80", "--json", "number,title,labels,assignees,comments,updatedAt"]),
      gh(["pr", "list", "-R", REPO, "--state", "open", "--json", "number,title,isDraft,statusCheckRollup"]),
      gh(["run", "list", "-R", REPO, "--branch", "main", "--workflow", "ci.yml", "--limit", "6", "--json", "status,conclusion"]),
    ]);
    const issues = JSON.parse(iss), pulls = JSON.parse(prs), runs = JSON.parse(runsJson);
    const lab = (i) => i.labels.map((l) => l.name);
    const logs = /^(Scout|Release|Ideator) log$/;
    const work = issues.filter((i) => !logs.test(i.title));
    const live = work.filter((i) => !lab(i).includes("hold"));
    const hasC = (i, s) => i.comments.some((c) => c.body.includes(s));
    const ready = live.filter((i) => !i.assignees.length && !lab(i).some((l) => l === "needs-decision" || l === "epic") &&
      (lab(i).includes("approved") || hasC(i, "triager + reviewer agree: ready")));
    const log = (t) => issues.find((i) => i.title === t);
    st.needs = findNeeds(live, lab);
    st.sel = Math.min(st.sel, Math.max(0, st.needs.length - 1));
    announce();
    st.up = {
      at: Date.now(),
      ready,
      inProgress: live.filter((i) => i.assignees.length).length,
      unplanned: live.filter((i) => !hasC(i, "## Plan") && !i.assignees.length).length,
      decisions: live.filter((i) => lab(i).includes("needs-decision")).length,
      mainCi: (() => {
        const done = runs.filter((r) => r.status === "completed").slice(0, 3).map((r) => r.conclusion);
        return done.length === 3 && done.every((c) => c === "failure") ? "red" : done[0] === "failure" ? "failing" : "green";
      })(),
      prs: pulls.map((p) => {
        const c = (p.statusCheckRollup ?? []).map((x) => x.conclusion || x.status);
        return { n: p.number, ci: c.some((x) => /FAIL|ERROR|CANCEL/i.test(x)) ? "failing" : c.length && c.every((x) => /SUCCESS|NEUTRAL|SKIPPED|COMPLETED/i.test(x)) ? "green" : "pending" };
      }),
      scoutAt: log("Scout log") ? Date.parse(log("Scout log").updatedAt) + THREE_H : null,
      ideatorAt: log("Ideator log") ? Date.parse(log("Ideator log").updatedAt) + THREE_H : null,
    };
  } catch (e) { st.up = { ...(st.up ?? {}), err: one(e.message).slice(0, 80), at: Date.now() }; if (process.env.AUTOPILOT_DEBUG) console.error(e); }
}

// ---- what needs the owner -----------------------------------------------------------------------
// Agent comments start with 🤖; anything else by the owner is the owner's own word.
const isAgent = (c) => c.body.trimStart().startsWith("🤖");
const strip = (s) => s.replace(/🤖\s*\*Claude:\*\s*(\*\*[^*]+\*\*)?/, "").trim();
function findNeeds(issues, lab) {
  const out = [];
  for (const i of issues) {
    const cs = i.comments;
    const answered = (t) => cs.some((c) => !isAgent(c) && c.author?.login === OWNER && c.createdAt > t);
    const lastAgent = [...cs].reverse().find(isAgent);
    const item = (kind, c, key, label) => ({ kind, key, n: i.number, title: i.title, label, text: strip(c?.body ?? "").slice(0, 1200) });
    if (lab(i).includes("needs-decision")) {
      if (answered(lastAgent?.createdAt ?? "")) continue;
      const plan = [...cs].reverse().find((c) => c.body.includes("## Plan")) ?? lastAgent;
      const body = strip(plan?.body ?? ""), at = body.search(/decision/i);
      out.push({ ...item("decision", plan, `d:${i.number}`, "decision"), text: (at >= 0 ? body.slice(at) : body).slice(0, 1200) });
      continue;
    }
    const w = [...cs].reverse().find((c) => /waiting for your (input|approval)/.test(c.body));
    if (w && !answered(w.createdAt)) { out.push(item("input", w, `i:${i.number}:${w.createdAt}`, "input")); continue; }
    const d = [...cs].reverse().find((c) => c.body.includes("Decision (autonomous)"));
    if (d && Date.now() - Date.parse(d.createdAt) < 48 * 3600e3 && !answered(d.createdAt) && !dismissed.has(`r:${i.number}:${d.createdAt}`))
      out.push(item("review", d, `r:${i.number}:${d.createdAt}`, "decided for you"));
  }
  return out;
}
function announce() {  // bell + notification for items we have not told the owner about yet
  const fresh = st.needs.filter((n) => n.kind !== "review" && !notified.has(n.key));
  fresh.forEach((n) => notified.add(n.key));
  if (!fresh.length || st.up === null) return;   // the first poll only seeds the set
  if (tty) process.stdout.write("\x07");
  execFile("osascript", ["-e", `display notification "${fresh.map((n) => `#${n.n}`).join(", ")} need your decision" with title "Margin autopilot"`], () => {});
}

const run = (cmd, args) => new Promise((res) => execFile(cmd, args, (e, out, err) => res({ ok: !e, out, err: one(err || e?.message) })));
const URL = (n) => `https://github.com/${REPO}/issues/${n}`;
function saveDismissed() { try { mkdirSync(STATE_DIR, { recursive: true }); writeFileSync(DISMISSED, JSON.stringify([...dismissed])); } catch {} }

async function act(action, item, text) {
  const n = String(item.n);
  let r = { ok: true }, done = "";
  if (action === "approve") {
    if (item.kind === "review") { dismissed.add(item.key); saveDismissed(); done = `kept the decision on #${n}`; }
    else {
      r = await run("gh", ["issue", "comment", n, "-R", REPO, "--body", text || "Approved by the owner (via the autopilot UI)."]);
      await run("gh", ["issue", "edit", n, "-R", REPO, "--add-label", "approved"]);
      await run("gh", ["issue", "edit", n, "-R", REPO, "--remove-label", "needs-decision"]);
      done = `approved #${n}`;
    }
  } else if (action === "reject") {
    r = await run("gh", ["issue", "comment", n, "-R", REPO, "--body", `Rejected by the owner (via the autopilot UI).${text ? ` ${text}` : ""}`]);
    if (r.ok) r = await run("gh", ["issue", "close", n, "-R", REPO, "--reason", "not planned"]);
    done = `closed #${n} as not planned`;
  } else if (action === "comment") {
    r = await run("gh", ["issue", "comment", n, "-R", REPO, "--body", text]);
    done = `commented on #${n}`;
  } else if (action === "hold") {
    await run("gh", ["label", "create", "hold", "-R", REPO, "--color", "b60205", "--description", "Owner veto: agents skip this"]);
    r = await run("gh", ["issue", "edit", n, "-R", REPO, "--add-label", "hold"]);
    done = `put #${n} on hold`;
  }
  st.msg = r.ok ? green(`✔ ${done}`) : red(`✖ ${done.split(" ")[0]} failed: ${r.err.slice(0, 120)}`);
  await pollUp();
}

// `n`: file a task for the agents as a normal issue (triager + reviewer gate as usual). Empty title cancels.
function queueTask(title = "") {
  st.input = { prompt: "Queue task: title", buf: title, cb: (t) => {
    st.input = { prompt: "Details (optional)", buf: "", empty: true, cb: async (body) => {
      st.msg = dim(`filing "${cut(t, 40)}"…`);
      const r = await run("gh", createArgs(REPO, t, body));
      if (r.ok) { st.msg = green(`✔ queued #${issueNo(r.out) || "?"} ${cut(t, 60)}`); pollUp(); }
      else { st.msg = red(`✖ queue failed: ${r.err.slice(0, 120)}`); queueTask(t); }
      render();
    } };
  } };
}

function onKey(buf) {
  const k = buf.toString();
  if (k === "\x03") { process.kill(0, "SIGINT"); return; }   // raw mode turns Ctrl-C into a plain key
  const it = st.needs[st.sel];
  if (st.input) {   // single-line text entry
    if (k === "\r") { const { cb, buf: text, empty } = st.input; st.input = null; if (text.trim() || empty) cb(text.trim()); }
    else if (k === "\x1b") st.input = null;
    else if (k === "\x7f") st.input.buf = st.input.buf.slice(0, -1);
    else if (!k.startsWith("\x1b")) st.input.buf += k.replace(/[\r\n]+/g, " ");
  } else if (st.confirm) {
    const c = st.confirm; st.confirm = null;
    if (k === "y") c();
  } else if (k === "n") queueTask();
  else if (!st.panel) {
    if (st.help) st.help = false;
    else if (k === "d") st.panel = true;
    else if (k === "?") st.help = true;
    else if (k === "j" || k === "\x1b[B") st.msel = Math.min(st.msel + 1, items.length - 1);
    else if (k === "k" || k === "\x1b[A") st.msel = Math.max(st.msel - 1, 0);
    else if (k === "o" && items[st.msel]) execFile("open", [items[st.msel].url], () => {});
  } else if (k === "d" || k === "q" || k === "\x1b") {
    if (standalone) process.exit(0);
    st.panel = false;
  } else if (k === "j" || k === "\x1b[B") st.sel = Math.min(st.sel + 1, st.needs.length - 1);
  else if (k === "k" || k === "\x1b[A") st.sel = Math.max(st.sel - 1, 0);
  else if (!it) { /* nothing selected */ }
  else if (k === "o") execFile("open", [URL(it.n)], () => {});
  else if (k === "a") act("approve", it);
  else if (k === "h") act("hold", it);
  else if (k === "x" && it.kind === "review") act("approve", it);
  else if (k === "c") st.input = { prompt: `Comment on #${it.n}`, buf: "", cb: (t) => act("comment", it, t) };
  else if (k === "r") {
    if (it.kind === "review") st.input = { prompt: `Overrule #${it.n}: what instead?`, buf: "", cb: (t) => act("comment", it, `Overruling the autonomous decision: ${t}`) };
    else st.confirm = () => act("reject", it);
  }
  render();
}

if (process.env.AUTOPILOT_DEBUG) globalThis.__key = onKey;   // lets tests press keys without a terminal

// ---- rendering ---------------------------------------------------------------------------------
const wrap = (s, w) => s.split("\n").flatMap((p) => { const o = []; let l = ""; for (const t of p.split(/\s+/)) { if ((l + " " + t).trim().length > w) { o.push(l); l = t; } else l = (l + " " + t).trim(); } o.push(l); return o; });
const inputLine = () => ` ${bold(st.input.prompt)}: ${st.input.buf}█   ${dim("Enter sends · Esc cancels")}`;
function renderPanel() {
  const w = Math.max(60, process.stdout.columns || 80), rows = process.stdout.rows || 40;
  const L = [bold(cyan(" Needs your decision")) + dim(`  ${st.needs.length} open`), dim("─".repeat(w))];
  if (!st.needs.length) L.push(dim("   nothing needs you right now"));
  st.needs.forEach((n, idx) => {
    const tag = n.kind === "review" ? dim(n.label) : yellow(n.label);
    L.push(`${idx === st.sel ? cyan(" ▶") : "  "} #${String(n.n).padEnd(4)} ${cut(n.title, w - 32).padEnd(Math.max(10, w - 32))} ${tag}`);
  });
  const it = st.needs[st.sel];
  if (it) {
    L.push(dim("─".repeat(w)), bold(` #${it.n} ${cut(it.title, w - 8)}`), dim(` ${URL(it.n)}`), "");
    for (const l of wrap(it.text, w - 4).slice(0, Math.max(4, rows - L.length - 7))) L.push(`  ${l}`);
  }
  L.push("");
  if (st.input) L.push(inputLine());
  else if (st.confirm) L.push(` ${yellow(bold(`Close #${it.n} as not planned?`))}  ${dim("y = yes, any other key cancels")}`);
  else {
    L.push(it?.kind === "review"
      ? ` ${bold("x")} keep decision · ${bold("r")} overrule · ${bold("c")} comment · ${bold("h")} hold · ${bold("o")} open · ${bold("↑↓")} select · ${bold("d")}/Esc back`
      : ` ${bold("a")} approve · ${bold("r")} reject · ${bold("c")} comment · ${bold("h")} hold · ${bold("o")} open · ${bold("↑↓")} select · ${bold("d")}/Esc back`);
    L.push(` ${bold("n")} queue a task for the agents`);
    if (st.msg) L.push(" " + st.msg);
  }
  process.stdout.write("\x1b[H\x1b[J" + L.join("\n"));
}
function resetText(ts, long) {
  if (!ts) return "";
  const ms = ts * 1000 - Date.now();
  const when = new Date(ts * 1000);
  return long
    ? `resets ${when.toLocaleDateString("en-GB", { weekday: "short" })} ${when.toTimeString().slice(0, 5)} (${dur(ms)})`
    : `resets in ${dur(ms)} (${when.toTimeString().slice(0, 5)})`;
}
function due(at, now) { return at == null ? "due now" : at <= now ? "due now" : `in ${dur(at - now)}`; }

let items = [];   // what j/k/o can act on in the main view, rebuilt on every render: Up next rows, then open PRs
const HELP = [
  ["d", "decisions: approve / reject / comment / hold (inside: a r c h x o, ↑↓)"],
  ["n", "queue a task for the agents (title, then details)"],
  ["j / k, ↑ / ↓", "select an Up next row or open PR"],
  ["o", "open the selected issue or PR in the browser"],
  ["?", "this help (any key closes it)"],
  ["Ctrl-C", "restart the runner"],
  ["touch ~/.cache/margin-autopilot/STOP", "stop after this run"],
];
function renderHelp() {
  const L = [bold(cyan(" Keys")), ""];
  for (const [k, d] of HELP) L.push(`  ${bold(k.padEnd(12))} ${d}`);
  process.stdout.write("\x1b[H\x1b[J" + L.join("\n"));
}

function render() {
  if (standalone || st.panel) return renderPanel();
  if (st.help) return renderHelp();
  const cols = process.stdout.columns || 80, w = Math.max(60, cols), rows = process.stdout.rows || 40;
  const wide = cols >= 120;                       // two columns: Agents + Up next | limits + Recent
  const cw = wide ? Math.floor((w - 3) / 2) : w, rw = w - 3 - cw;
  const spin = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"[Math.floor(Date.now() / 100) % 10];
  const now = Date.now();
  const live = [...agents.values()].filter((a) => now - a.seen < 15 * 60e3).sort((a, b) => b.seen - a.seen);
  const active = live.filter((a) => now - a.seen < 30e3).length;
  const head = [];
  head.push(bold(cyan(" Margin autopilot")) + dim(`  up ${dur(now - started)}${st.model ? `  ·  ${st.model}` : ""}`));
  head.push(dim("─".repeat(w)));
  const todo = st.needs.filter((n) => n.kind !== "review"), rev = st.needs.length - todo.length;
  if (todo.length) {
    head.push(yellow(bold(` ⚠  ${todo.length} need${todo.length === 1 ? "s" : ""} your decision`)) + dim(`  — press `) + bold("d") + dim(" to approve / reject / comment"));
    for (const n of todo.slice(0, 3)) head.push(`    ${yellow("▸")} #${n.n} ${cut(n.title, w - 22)} ${dim(n.label)}`);
    if (todo.length > 3) head.push(dim(`    … and ${todo.length - 3} more`));
  } else if (rev) head.push(dim(` ${rev} autonomous decision${rev === 1 ? "" : "s"} to review (optional) — press `) + bold("d"));
  else head.push(green(" ✔ nothing needs you"));
  if (st.input) head.push(inputLine());
  if (st.msg) head.push(" " + st.msg);
  head.push(dim("─".repeat(w)));
  head.push(sleepUntil
    ? ` 💤 ${bold("Sleeping")}  ${cut(sleepWhy, w - 40)} - next run in ${dur(sleepUntil - now)}`
    : ` ${cyan(spin)} ${bold("Now")}  ${cut(st.now, w - 10)}`);

  // usage + context
  const lim = st.limits, win = lim?.unifiedWindows ?? {};
  const fh = win.five_hour, sd = win.seven_day, bw = wide ? 12 : 20;
  const limRow = (name, x, long) => x
    ? ` ${name.padEnd(8)} ${bar(x.utilization, bw)} ${String(Math.round(x.utilization * 100)).padStart(3)}%  ${dim(resetText(x.resetsAt, long))}`
    : ` ${name.padEnd(8)} ${dim("waiting for first API reply…")}`;
  const cf = st.ctx / CTX;
  const limits = [
    limRow("5h limit", fh, false),
    limRow("Weekly", sd, true) + (lim && lim.status !== "allowed" ? "  " + red(bold(lim.status === "rejected" ? "LIMIT HIT" : lim.status)) : ""),
    ` ${"Context".padEnd(8)} ${bar(cf, bw)} ${String(Math.round(cf * 100)).padStart(3)}%  ${dim(`${tok(st.ctx)} / ${tok(CTX)}${wide ? "" : " coordinator"}${st.compactions ? ` · ${st.compactions} compaction${st.compactions > 1 ? "s" : ""}` : ""}`)}`,
  ];

  // agents (not selectable: names carry no reliable issue number)
  const nameW = wide ? 20 : 30;
  const agentL = [bold(" Agents") + dim(`  ${active}/${MAX} slots busy · ${st.agents} started this run`)];
  if (!live.length) agentL.push(dim("   none running"));
  for (const a of live.slice(0, 5)) {
    const dot = now - a.seen < 30e3 ? green("●") : yellow("●");
    agentL.push(`   ${dot} ${cut(a.name, nameW).padEnd(nameW)} ${dim(dur(now - a.born).padStart(7))}  ${dim(cut(a.last, cw - nameW - 16))}`);
  }

  // up next
  const u = st.up, upL = [];
  const nReady = u?.ready ? Math.min(3, Math.max(0, MAX - active) || 1, u.ready.length) : 0;
  items = u?.ready ? [...u.ready.slice(0, nReady).map((i) => ({ url: URL(i.number) })), ...u.prs.map((p) => ({ url: `https://github.com/${REPO}/pull/${p.n}` }))] : [];
  st.msel = Math.min(st.msel, Math.max(0, items.length - 1));
  upL.push(bold(" Up next") + (u?.err ? red(`  gh error: ${u.err}`) : u ? dim(`  from GitHub, ${dur(now - u.at)} ago`) : dim("  loading…")));
  if (u?.ready) {
    const free = Math.max(0, MAX - active);
    upL.push(`   ${bold(u.ready.length)} ready to build · ${u.inProgress} in progress · ${free} free slot${free === 1 ? "" : "s"}`);
    u.ready.slice(0, nReady).forEach((i, k) => upL.push(`   ${k === st.msel ? cyan("▶") : cyan("▸")} ${k === st.msel ? readyRow(i, cw - 6) : dim(readyRow(i, cw - 6))}`));
    const pr = (p, k) => { const s = `#${p.n} ${p.ci === "green" ? green("green") : p.ci === "failing" ? red("failing") : yellow("pending")}`; return k + nReady === st.msel ? bold(cyan("▶")) + s : s; };
    upL.push(`   ${bold(u.unplanned)} to triage · PRs: ${u.prs.length ? u.prs.map(pr).join(", ") : dim("none open")}`);
    const full = u.ready.length >= 2 * MAX;
    upL.push(`   main CI ${u.mainCi === "green" ? green("green") : u.mainCi === "failing" ? yellow("last run failed") : red(bold("RED, 3 failures in a row"))}`);
    upL.push(`   scout ${dim(due(u.scoutAt, now))} · ideator ${full ? dim(`idle, queue ≥ ${2 * MAX}`) : dim(due(u.ideatorAt, now))} · triager ${dim("watching")}`);
  }

  const totalsL = [` ${bold(st.issues)} issues${wide ? "" : " filed"} · ${bold(st.prs)} PRs${wide ? "" : " opened"} · ${green(bold(st.merges))} merged · ${st.commits} commits · ${dim(st.tools + (wide ? " tools" : " tool calls"))}`];
  const recentL = (n, w2) => [bold(" Recent"), ...st.recent.slice(-n).map((r) => {
    const txt = cut(`${r.who ? `${cut(r.who, 18)}: ` : ""}${r.line}`, w2 - 14);
    return `  ${dim(clock(r.t))} ${r.kind === "text" ? txt : dim(txt)}`;
  })];
  const more = (n) => dim(`   (+${n} more)`);

  let body;
  if (wide) {   // left: Agents, Up next, totals; right: limits, Recent. Each column fills the rows below the head.
    const avail = rows - head.length - 3;   // minus the blank line above the columns, and blank + footer below
    const left = budget(avail - agentL.length - 1, upL.length, 2, 0, 0);
    const right = budget(avail - limits.length - 2, 0, 0, 3, 14);
    body = ["", ...columns(
      [...agentL, "", ...clip(upL, left.up, more), ...(left.totals ? ["", ...totalsL] : [])],
      [...limits, "", ...recentL(right.recent, rw)], cw)];
  } else {
    const fixed = head.length + 1 + limits.length + 1 + agentL.length + 1 + 1 + 1 + 2;   // blanks, rule, Recent title, footer
    const b = budget(rows - fixed, upL.length, 2, 3, 8);
    body = ["", ...limits, "", ...agentL, "", ...clip(upL, b.up, more), ...(b.totals ? ["", ...totalsL] : []), dim("─".repeat(w)), ...recentL(b.recent, w)];
  }
  const hints = [todo.length || rev ? "d: decisions" : "", "n: queue task", items.length ? "j/k: select · o: open" : "", "?: help"].filter(Boolean).join("  ·  ");
  process.stdout.write("\x1b[H\x1b[J" + [...head, ...body, "", dim(cut(` ${hints}  ·  Ctrl-C: restart`, w))].join("\n"));
}

// End the run for a fresh coordinator context. Soft: ≥70% while no agent is busy. Hard: ≥90%.
// Everything is in GitHub/git, so the next run adopts unfinished work.
function checkRotate() {
  if (st.rotating || !SESSION || !st.ctx || Date.now() - started < (Number(process.env.AUTOPILOT_ROTATE_AFTER) || 300) * 1e3) return;
  const f = st.ctx / CTX;
  const busy = [...agents.values()].some((a) => Date.now() - a.seen < 60e3);
  if (f < 0.9 && !(f >= 0.7 && !busy)) return;
  st.rotating = true;
  st.now = `rotating: coordinator context at ${Math.round(f * 100)}%, restarting with a fresh one`;
  note(st.now, null, "done");
  if (ROTATE) { try { writeFileSync(ROTATE, String(Date.now())); } catch {} }
  // pkill never signals its own parent (us), so exit ourselves once claude is stopped
  execFile("pkill", ["-f", "--", SESSION], () => { writeTail(); process.exit(143); });
}

pollUp();
setInterval(pollUp, 60e3).unref();
setInterval(checkRotate, 10e3).unref();
if (tty) {
  process.stdout.write("\x1b[?1049h\x1b[?25l"); // alternate screen, hide cursor
  const leave = () => process.stdout.write("\x1b[?25h\x1b[?1049l");
  process.on("exit", leave);
  process.on("SIGINT", () => process.exit(130));
  setInterval(render, 250).unref();
  // keys come from the terminal itself: our stdin is the claude stream
  try {
    const keys = new ReadStream(openSync("/dev/tty", "r"));
    keys.setRawMode(true);
    keys.on("data", onKey);
    process.on("exit", () => { try { keys.setRawMode(false); } catch {} });
    if (standalone) keys.ref(); else keys.unref();
  } catch { /* no controlling terminal (launchd): decisions are done with --decide elsewhere */ }
}
const writeTail = () => { if (TAIL) { try { writeFileSync(TAIL, rawTail.join("\n") + "\n"); } catch {} } };
// The runner/rotation stop the run with `pkill -f <session>`; the session id is also our argv[2], so we get SIGTERM too.
process.on("SIGTERM", () => { writeTail(); process.exit(143); });

if (sleepUntil) {   // no run attached: show limits from the last run and stay up until the deadline or STOP
  try { const l = JSON.parse(readFileSync(limitsFile, "utf8")); st.limits = { ...l, seen: l.ts }; } catch {}
  process.stdin.destroy();
  setInterval(() => { if (Date.now() >= sleepUntil || existsSync(join(STATE_DIR, "STOP"))) process.exit(0); }, 1000);
  render();
} else if (standalone) {   // no run attached: the key stream keeps the process alive
  process.stdin.destroy();
  render();
} else {
  const rl = createInterface({ input: process.stdin });
  rl.on("line", (l) => {
    rawTail.push(l.slice(0, 500));
    if (rawTail.length > 20) rawTail.shift();
    if (HEARTBEAT && Date.now() - lastBeat > 5e3) { lastBeat = Date.now(); try { writeFileSync(HEARTBEAT, ""); } catch {} }
    try { onEvent(JSON.parse(l)); }
    catch { note(l, null, "raw"); }
    if (tty) render();
  });
  rl.on("close", () => {
    writeTail();
    if (tty) render();
    const sum = `run ended after ${dur(Date.now() - started)}: ${st.issues} issues, ${st.prs} PRs opened, ${st.merges} merged, ${st.tools} tool calls`;
    process.on("exit", () => console.log(`\n${sum}`));
  });
}
