#!/usr/bin/env node
// Live dashboard for scripts/autopilot-forever.sh: reads `claude --output-format stream-json`
// on stdin and shows what is happening right now. Plain timestamped lines when stdout is not a TTY.
// Set AUTOPILOT_LOG to also append plain lines to a file.
import { createInterface } from "node:readline";
import { appendFileSync } from "node:fs";
import { basename } from "node:path";

const tty = process.stdout.isTTY;
const logFile = process.env.AUTOPILOT_LOG;
const sgr = (n) => (s) => (tty ? `\x1b[${n}m${s}\x1b[0m` : s);
const [bold, dim, red, green, yellow, cyan] = [1, 2, 31, 32, 33, 36].map(sgr);

const started = Date.now();
const st = { tools: 0, agents: 0, issues: 0, prs: 0, merges: 0, commits: 0, model: "", now: "starting…", recent: [] };
const agents = new Map(); // Agent tool_use id -> { name, born, seen, last }

const clock = (t = Date.now()) => new Date(t).toTimeString().slice(0, 8);
const dur = (ms) => {
  const s = Math.floor(ms / 1000), h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60);
  return h ? `${h}h${String(m).padStart(2, "0")}m` : `${m}m${String(s % 60).padStart(2, "0")}s`;
};
const cut = (s, n) => (n < 2 ? "" : s.length > n ? s.slice(0, n - 1) + "…" : s);
const one = (s) => String(s ?? "").replace(/\s+/g, " ").trim();

function describe(b) {
  const i = b.input ?? {};
  const arg = /^(Read|Edit|Write)$/.test(b.name) ? basename(i.file_path ?? "") : i.description ?? i.command ?? i.pattern ?? i.prompt ?? i.url ?? "";
  return `${b.name} ${one(arg)}`.trim();
}

function count(b) {
  st.tools++;
  const cmd = b.input?.command ?? "";
  if (b.name === "Agent") { st.agents++; }
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
  if (e.type === "system" && e.model) st.model = e.model;
  else if (e.type === "assistant") {
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

function render() {
  const w = Math.max(50, process.stdout.columns || 80);
  const spin = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"[Math.floor(Date.now() / 100) % 10];
  const now = Date.now();
  const live = [...agents.values()].filter((a) => now - a.seen < 15 * 60e3).sort((a, b) => b.seen - a.seen);
  const L = [];
  L.push(bold(cyan(" Margin autopilot")) + dim(`  up ${dur(now - started)}${st.model ? `  ·  ${st.model}` : ""}`));
  L.push(dim("─".repeat(w)));
  L.push(` ${cyan(spin)} ${bold("Now")}  ${cut(st.now, w - 10)}`);
  L.push("");
  L.push(bold(" Agents") + dim(`  ${live.filter((a) => now - a.seen < 30e3).length} active · ${st.agents} started this run`));
  if (!live.length) L.push(dim("   none running"));
  for (const a of live.slice(0, 6)) {
    const dot = now - a.seen < 30e3 ? green("●") : yellow("●");
    L.push(`   ${dot} ${cut(a.name, 30).padEnd(30)} ${dim(dur(now - a.born).padStart(7))}  ${dim(cut(a.last, w - 46))}`);
  }
  L.push("");
  L.push(` ${bold(st.issues)} issues filed · ${bold(st.prs)} PRs opened · ${green(bold(st.merges))} merged · ${st.commits} commits · ${dim(st.tools + " tool calls")}`);
  L.push(dim("─".repeat(w)));
  L.push(bold(" Recent"));
  for (const r of st.recent.slice(-8)) {
    const who = r.who ? `${cut(r.who, 18)}: ` : "";
    const txt = cut(`${who}${r.line}`, w - 14);
    L.push(`  ${dim(clock(r.t))} ${r.kind === "text" ? txt : dim(txt)}`);
  }
  L.push("", dim(" Ctrl-C: restart  ·  touch ~/.cache/margin-autopilot/STOP: stop after this run"));
  process.stdout.write("\x1b[H\x1b[J" + L.join("\n"));
}

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
  if (tty) { render(); }
  const sum = `run ended after ${dur(Date.now() - started)}: ${st.issues} issues, ${st.prs} PRs opened, ${st.merges} merged, ${st.tools} tool calls`;
  process.on("exit", () => console.log(sum));
});
