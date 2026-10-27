// Run: npm test
import { test } from "node:test";
import assert from "node:assert/strict";
import { markdown, mediaUrl, selected } from "./pr-media.ts";

test("selected keeps everything without a filter and only the named shots with one", () => {
  const all = ["note", "graph", "today"];
  assert.deepEqual(selected(all), all);
  assert.deepEqual(selected(all, " graph, today ,nope"), ["graph", "today"]);
});

test("mediaUrl and markdown point at the media branch", () => {
  assert.equal(mediaUrl(7, "a.png"), "https://raw.githubusercontent.com/albin02forsberg/margin/media/pr-7/a.png");
  assert.equal(markdown(7, ["a.png", "b.gif"]), `![a](${mediaUrl(7, "a.png")})\n\n![b](${mediaUrl(7, "b.gif")})`);
});
