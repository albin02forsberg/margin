#!/usr/bin/env node
// Live dashboard for scripts/autopilot-forever.sh: reads `claude --output-format stream-json`
// on stdin and shows what is happening now, usage limits, context and what comes next.
// Plain timestamped lines when stdout is not a TTY.
// Env: AUTOPILOT_LOG (append plain lines), AUTOPILOT_LIMITS (write last rate-limit info as JSON),
//      AUTOPILOT_MAX (agent slots, default 3), AUTOPILOT_CONTEXT (context window tokens, default 200000),
//      AUTOPILOT_REPO (default albin02forsberg/margin).
import { createInterface } from "node:readline";
import { appendFileSync, writeFileSync } from "node:fs";
import { execFile } from "node:child_process";
import { basename } from "node:path";

const tty = process.stdout.isTTY;
const logFile = process.env.AUTOPILOT_LOG;
const limitsFile = process.env.AUTOPILOT_LIMITS;
const MAX = Number(process.env.AUTOPILOT_MAX) || 3;
const CTX = Number(process.env.AUTOPILOT_CONTEXT) || 200000;
const REPO = process.env.AUTOPILOT_REPO || "albin02forsberg/margin";
const sgr = (n) => (s) => (tty ? `\x1b[${n}m${s}\x1b[0m` : s);
const [bold, dim, red, green, yellow, cyan] = [1, 2, 31, 32, 33, 36].map(sgr);

const started = Date.now();
const st = {
  tools: 0, agents: 0, issues: 0, prs: 0, merges: 0, commits: 0, model: "", now: "starting…", recent: [],
  ctx: 0, compactions: 0, limits: null, up: null,
};
const agents = new Map(); // Agent tool_use id -> { name, born, seen, last }

const clock = (t = Date.now()) => new Date(t).toTimeString().slice(0, 8);
const dur = (ms) => {
  const s = Math.max(0, Math.floor(ms / 1000)), d = Math.floor(s / 86400), h = Math.floor((s % 86400) / 3600), m = Math.floor((s % 3600) / 60);
  return d ? `${d}d${h}h` : h ? `${h}h${String(m).padStart(2, "0")}m` : `${m}m${String(s % 60).padStart(2, "0")}s`;
};
const cut = (s, n) => (n < 2 ? "" : s.length > n ? s.slice(0, n - 1) + "…" : s);
const one = (s) => String(s ?? "").replace(/\s+/g, " ").trim();
const tok = (n) => (n >= 1000 ? `${Math.round(n / 1000)}k` : String(n));
const bar = (f, w = 20) => {
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
  if (!tty) console.log(`${clock()} ${a ? `  ↳ ${a.name}: ` : ""}${line}`);
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
    const [iss, prs] = await Promise.all([
      gh(["issue", "list", "-R", REPO, "--state", "open", "--limit", "80", "--json", "number,title,labels,assignees,comments,updatedAt"]),
      gh(["pr", "list", "-R", REPO, "--state", "open", "--json", "number,title,isDraft,statusCheckRollup"]),
    ]);
    const issues = JSON.parse(iss), pulls = JSON.parse(prs);
    const lab = (i) => i.labels.map((l) => l.name);
    const logs = /^(Scout|Release|Ideator) log$/;
    const work = issues.filter((i) => !logs.test(i.title));
    const live = work.filter((i) => !lab(i).includes("hold"));
    const hasC = (i, s) => i.comments.some((c) => c.body.includes(s));
    const ready = live.filter((i) => !i.assignees.length && !lab(i).some((l) => l === "needs-decision" || l === "epic") &&
      (lab(i).includes("approved") || hasC(i, "triager + reviewer agree: ready")));
    const log = (t) => issues.find((i) => i.title === t);
    st.up = {
      at: Date.now(),
      ready,
      inProgress: live.filter((i) => i.assignees.length).length,
      unplanned: live.filter((i) => !hasC(i, "## Plan") && !i.assignees.length).length,
      decisions: live.filter((i) => lab(i).includes("needs-decision")).length,
      prs: pulls.map((p) => {
        const c = (p.statusCheckRollup ?? []).map((x) => x.conclusion || x.status);
        return { n: p.number, ci: c.some((x) => /FAIL|ERROR|CANCEL/i.test(x)) ? "failing" : c.length && c.every((x) => /SUCCESS|NEUTRAL|SKIPPED|COMPLETED/i.test(x)) ? "green" : "pending" };
      }),
      scoutAt: log("Scout log") ? Date.parse(log("Scout log").updatedAt) + THREE_H : null,
      ideatorAt: log("Ideator log") ? Date.parse(log("Ideator log").updatedAt) + THREE_H : null,
    };
  } catch (e) { st.up = { ...(st.up ?? {}), err: one(e.message).slice(0, 80), at: Date.now() }; }
}

// ---- rendering ---------------------------------------------------------------------------------
function resetText(ts, long) {
  if (!ts) return "";
  const ms = ts * 1000 - Date.now();
  const when = new Date(ts * 1000);
  return long
    ? `resets ${when.toLocaleDateString("en-GB", { weekday: "short" })} ${when.toTimeString().slice(0, 5)} (${dur(ms)})`
    : `resets in ${dur(ms)} (${when.toTimeString().slice(0, 5)})`;
}
function due(at, now) { return at == null ? "due now" : at <= now ? "due now" : `in ${dur(at - now)}`; }

function render() {
  const w = Math.max(60, process.stdout.columns || 80);
  const rows = process.stdout.rows || 40;
  const spin = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"[Math.floor(Date.now() / 100) % 10];
  const now = Date.now();
  const live = [...agents.values()].filter((a) => now - a.seen < 15 * 60e3).sort((a, b) => b.seen - a.seen);
  const active = live.filter((a) => now - a.seen < 30e3).length;
  const L = [];
  L.push(bold(cyan(" Margin autopilot")) + dim(`  up ${dur(now - started)}${st.model ? `  ·  ${st.model}` : ""}`));
  L.push(dim("─".repeat(w)));
  L.push(` ${cyan(spin)} ${bold("Now")}  ${cut(st.now, w - 10)}`);

  // usage + context
  L.push("");
  const lim = st.limits, win = lim?.unifiedWindows ?? {};
  const fh = win.five_hour, sd = win.seven_day;
  const limRow = (name, x, long) => x
    ? ` ${name.padEnd(8)} ${bar(x.utilization)} ${String(Math.round(x.utilization * 100)).padStart(3)}%  ${dim(resetText(x.resetsAt, long))}`
    : ` ${name.padEnd(8)} ${dim("waiting for first API reply…")}`;
  L.push(limRow("5h limit", fh, false));
  L.push(limRow("Weekly", sd, true) + (lim && lim.status !== "allowed" ? "  " + red(bold(lim.status === "rejected" ? "LIMIT HIT" : lim.status)) : ""));
  const cf = st.ctx / CTX;
  L.push(` ${"Context".padEnd(8)} ${bar(cf)} ${String(Math.round(cf * 100)).padStart(3)}%  ${dim(`${tok(st.ctx)} / ${tok(CTX)} coordinator${st.compactions ? ` · ${st.compactions} compaction${st.compactions > 1 ? "s" : ""}` : ""}`)}`);

  // agents
  L.push("");
  L.push(bold(" Agents") + dim(`  ${active}/${MAX} slots busy · ${st.agents} started this run`));
  if (!live.length) L.push(dim("   none running"));
  for (const a of live.slice(0, 5)) {
    const dot = now - a.seen < 30e3 ? green("●") : yellow("●");
    L.push(`   ${dot} ${cut(a.name, 30).padEnd(30)} ${dim(dur(now - a.born).padStart(7))}  ${dim(cut(a.last, w - 46))}`);
  }

  // up next
  L.push("");
  const u = st.up;
  L.push(bold(" Up next") + (u?.err ? red(`  gh error: ${u.err}`) : u ? dim(`  from GitHub, ${dur(now - u.at)} ago`) : dim("  loading…")));
  if (u?.ready) {
    const free = Math.max(0, MAX - active);
    L.push(`   ${bold(u.ready.length)} ready to build · ${u.inProgress} in progress · ${free} free slot${free === 1 ? "" : "s"}`);
    for (const i of u.ready.slice(0, Math.min(3, free || 1))) L.push(`   ${cyan("▸")} #${i.number} ${dim(cut(i.title, w - 14))}`);
    L.push(`   ${bold(u.unplanned)} to triage · ${u.decisions} awaiting a decision · PRs: ${u.prs.length ? u.prs.map((p) => `#${p.n} ${p.ci === "green" ? green("green") : p.ci === "failing" ? red("failing") : yellow("pending")}`).join(", ") : dim("none open")}`);
    const full = u.ready.length >= 2 * MAX;
    L.push(`   scout ${dim(due(u.scoutAt, now))} · ideator ${full ? dim(`idle, queue ≥ ${2 * MAX}`) : dim(due(u.ideatorAt, now))} · triager ${dim("watching")}`);
  }

  L.push("");
  L.push(` ${bold(st.issues)} issues filed · ${bold(st.prs)} PRs opened · ${green(bold(st.merges))} merged · ${st.commits} commits · ${dim(st.tools + " tool calls")}`);
  L.push(dim("─".repeat(w)));
  const room = Math.max(3, Math.min(8, rows - L.length - 3));
  L.push(bold(" Recent"));
  for (const r of st.recent.slice(-room)) {
    const txt = cut(`${r.who ? `${cut(r.who, 18)}: ` : ""}${r.line}`, w - 14);
    L.push(`  ${dim(clock(r.t))} ${r.kind === "text" ? txt : dim(txt)}`);
  }
  L.push("", dim(" Ctrl-C: restart  ·  touch ~/.cache/margin-autopilot/STOP: stop after this run"));
  process.stdout.write("\x1b[H\x1b[J" + L.join("\n"));
}

pollUp();
setInterval(pollUp, 60e3).unref();
if (tty) {
  process.stdout.write("\x1b[?1049h\x1b[?25l"); // alternate screen, hide cursor
  const leave = () => process.stdout.write("\x1b[?25h\x1b[?1049l");
  process.on("exit", leave);
  process.on("SIGINT", () => process.exit(130));
  setInterval(render, 250).unref();
}

const rl = createInterface({ input: process.stdin });
rl.on("line", (l) => {
  try { onEvent(JSON.parse(l)); }
  catch { note(l, null, "raw"); }
  if (tty) render();
});
rl.on("close", () => {
  if (tty) render();
  const sum = `run ended after ${dur(Date.now() - started)}: ${st.issues} issues, ${st.prs} PRs opened, ${st.merges} merged, ${st.tools} tool calls`;
  process.on("exit", () => console.log(`\n${sum}`));
});
