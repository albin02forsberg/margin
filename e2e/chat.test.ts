// End-to-end test of the AI chat panel (#205) against a stand-in Ollama on loopback.
import { after, before, describe, it } from "node:test";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import assert from "node:assert/strict";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { answer, browser, close, find, Key, launch, type } from "./setup.ts";

describe("AI chat", () => {
  let ollama: Server;
  const asked: { path: string; body: any }[] = [];
  before(async () => {
    ollama = createServer((req, res) => {
      let s = "";
      req.on("data", (c) => (s += c)).on("end", () => {
        asked.push({ path: req.url!, body: JSON.parse(s) });
        res.writeHead(200, { "Content-Type": "application/json" }).end(JSON.stringify({ message: { role: "assistant", content: `Answer ${asked.length}` }, done: true }));
      });
    });
    await new Promise<void>((r) => ollama.listen(0, "127.0.0.1", r));
    await launch({
      config: [`ai_backend = "ollama"`, `ai_model = "m"`, `ai_url = "http://127.0.0.1:${(ollama.address() as AddressInfo).port}"`],
      files: ({ notes }) => writeFileSync(join(notes, "plans.org"), "#+title: Plans\n\n* TODO Ship the chat\n"),
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
    assert.equal(asked[0].path, "/api/chat");
    const [sys, ...rest] = asked[1].body.messages;
    assert.ok(sys.role === "system" && sys.content.includes("* TODO Ship the chat"), sys.content);
    assert.deepEqual(rest, [
      { role: "user", content: "What's next?" },
      { role: "assistant", content: "Answer 1" },
      { role: "user", content: "And then?" },
    ]);
  });
});
