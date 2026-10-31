// End-to-end test of the AI chat panel (#205) against a stand-in Ollama on loopback.
import { after, before, describe, it } from "node:test";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import assert from "node:assert/strict";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { answer, browser, close, fileMatches, find, Key, launch, texts, type, type Dirs } from "./setup.ts";

describe("AI chat", () => {
  let ollama: Server;
  let d: Dirs;
  const asked: { path: string; body: any }[] = [];
  before(async () => {
    ollama = createServer((req, res) => {
      let s = "";
      req.on("data", (c) => (s += c)).on("end", () => {
        const body = JSON.parse(s);
        asked.push({ path: req.url!, body });
        // Streamed in pieces (#210); a question with "Stall" never finishes, so only Cancel ends it.
        const line = (content: string, done = false) => JSON.stringify({ message: { role: "assistant", content }, done }) + "\n";
        // With tools (#963): "Notool" is a model that can't use them; else one search, then the answer.
        const q = body.messages.findLast((m: any) => m.role === "user")?.content ?? "";
        if (body.tools && q.includes("Notool")) {
          res.writeHead(400, { "Content-Type": "application/json" });
          return void res.end(JSON.stringify({ error: "registry.ollama.ai/library/m does not support tools" }));
        }
        res.writeHead(200, { "Content-Type": "application/x-ndjson" });
        if (body.tools && body.messages.at(-1).role !== "tool") {
          return void res.end(JSON.stringify({ message: { role: "assistant", content: "", tool_calls: [{ function: { name: "search_notes", arguments: { query: "rollout" } } }] }, done: true }) + "\n");
        }
        if (body.messages.at(-1).content.includes("Stall")) return void res.write(line("Partial"));
        res.write(line("Answer "));
        res.end(line(String(asked.length), true));
      });
    });
    await new Promise<void>((r) => ollama.listen(0, "127.0.0.1", r));
    d = await launch({
      config: [`ai_backend = "ollama"`, `ai_model = "m"`, `ai_url = "http://127.0.0.1:${(ollama.address() as AddressInfo).port}"`],
      files: ({ notes }) => {
        writeFileSync(join(notes, "plans.org"), "#+title: Plans\n\n* TODO Ship the chat\nSee [[file:launch.org][the launch]].\n");
        writeFileSync(join(notes, "launch.org"), "#+title: Launch\n\n* Date\nThe rollout happens on May 3.\n");
      },
    });
  });
  after(async () => { await close(); ollama.close(); }, { timeout: 30_000 });

  it("answers about the open note and keeps the history", async () => {
    await find(".agenda h1", "Today");
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "Plans");
    await find(".status .file", "plans.org");
    await browser.keys(" ");
    await type("na");
    await find(".chat label", "Include");
    await type("What's next?");
    await browser.keys(Key.Enter);
    await find(".chat p.assistant", "Answer 1");
    await type("And then?");
    await browser.keys(Key.Enter);
    await find(".chat p.assistant", "Answer 2");
    await (await find(".chat .acts button", "Insert")).click();
    await find(".cm-content", "Answer 1");
    assert.equal(asked[0].path, "/api/chat");
    const [sys, ...rest] = asked[1].body.messages;
    assert.ok(sys.role === "system" && sys.content.includes("* TODO Ship the chat"), sys.content);
    assert.deepEqual(rest, [
      { role: "user", content: "What's next?" },
      { role: "assistant", content: "Answer 1" },
      { role: "user", content: "And then?" },
    ]);
  });

  it("cancels an answer midway and keeps what came", async () => {
    await (await find(".chat textarea", "")).click();
    await type("Stall please");
    await browser.keys(Key.Enter);
    await find(".chat p.assistant", "Partial");
    await (await find(".status .cancel", "cancel")).click();
    await find(".chat .err", "Cancelled.");
    await find(".chat p.assistant", "Partial");
  });

  it("answers from the matching and linked notes and lists them (#304)", async () => {
    await (await find(".chat label", "Use my notes")).click();
    await (await find(".chat textarea", "")).click();
    const n = asked.length;
    await type("When is the rollout?");
    await browser.keys(Key.Enter);
    await find(".chat p.assistant", `Answer ${n + 1}`);
    const sys = asked.at(-1)!.body.messages[0].content;
    assert.ok(sys.includes("[note: Launch | launch.org:3]\n* Date\nThe rollout happens on May 3."), sys);
    await (await find(".chat .srcs button", "Launch:3")).click();
    await find(".status .file", "launch.org");
  });

  it("sends open tasks when asked about them, unless switched off in Settings (#468)", async () => {
    await (await find(".chat textarea", "")).click();
    await type("Which tasks are open?");
    await browser.keys(Key.Enter);
    await find(".chat .srcs span", "Tasks");
    let sys = asked.at(-1)!.body.messages[0].content;
    assert.ok(sys.includes("[tasks: open tasks]\n- TODO Ship the chat (plans.org)"), sys);
    await browser.keys([Key.Ctrl, ","]);
    await (await browser.$(`//div[contains(@class, "settings")]//label[span[contains(., "Send open tasks")]]/input`)).click();
    await (await find(".settings button", "Save ai chat")).click();
    await fileMatches(join(d.cfgDir, "config.toml"), (s) => /^ai_source_tasks = false$/m.test(s), "switch not saved");
    await find(".status .msg", "Settings saved");
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "Plans");
    await find(".status .file", "plans.org");
    await (await find(".chat textarea", "")).click();
    const n = asked.length;
    await type("Which tasks are left?");
    await browser.keys(Key.Enter);
    await find(".chat p.assistant", `Answer ${n + 1}`);
    sys = asked.at(-1)!.body.messages[0].content;
    assert.ok(!sys.includes("[tasks: open tasks]"), sys);
    assert.equal((await texts(".chat .srcs span")).length, 1, "only the first answer lists Tasks");
  });

  it("lets the model search notes with tools, and says when it can't (#963)", async () => {
    await browser.keys([Key.Ctrl, ","]);
    await (await browser.$(`//div[contains(@class, "settings")]//label[span[contains(., "Let the model look things up itself")]]/input`)).click();
    await (await find(".settings button", "Save ai chat")).click();
    await fileMatches(join(d.cfgDir, "config.toml"), (s) => /^ai_tools = true$/m.test(s), "switch not saved");
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "Plans");
    await find(".status .file", "plans.org");
    await (await find(".chat textarea", "")).click();
    let n = asked.length;
    await type("When is the rollout?");
    await browser.keys(Key.Enter);
    await find(".chat .tool", "searched notes: rollout");
    await find(".chat p.assistant", `Answer ${n + 2}`);
    const [call, result] = asked.slice(n);
    // tasks and the time log are on by default, so their tools come too (#990)
    const offered = (call.body.tools ?? []).map((t: any) => t.function.name);
    assert.ok(["search_notes", "list_tasks", "agenda", "time_report"].every((n) => offered.includes(n)), JSON.stringify(call.body));
    const tool = result.body.messages.at(-1);
    assert.ok(tool.role === "tool" && tool.content.includes("The rollout happens on May 3."), JSON.stringify(tool));
    n = asked.length;
    await type("Notool: when is the rollout?");
    await browser.keys(Key.Enter);
    await find(".chat .tool", "can't use tools");
    await find(".chat p.assistant", `Answer ${n + 2}`);
    assert.ok(!asked.at(-1)!.body.tools, "the fallback sends no tools");
  });
});
