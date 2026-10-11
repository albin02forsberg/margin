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
        const body = JSON.parse(s);
        asked.push({ path: req.url!, body });
        // Streamed in pieces (#210); a question with "Stall" never finishes, so only Cancel ends it.
        const line = (content: string, done = false) => JSON.stringify({ message: { role: "assistant", content }, done }) + "\n";
        res.writeHead(200, { "Content-Type": "application/x-ndjson" });
        if (body.messages.at(-1).content.includes("Stall")) return void res.write(line("Partial"));
        res.write(line("Answer "));
        res.end(line(String(asked.length), true));
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
});
