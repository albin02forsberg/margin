// PR/issue media (#726): screenshots of a feature live on the orphan `media` branch (never merged,
// so main stays small) as pr-<n>/<name>.png|gif; markdown embeds the raw.githubusercontent URL.
// Usage: node scripts/pr-media.ts <pr> <files...> [--body]   (--body appends a ## Preview section to the PR)
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { fileURLToPath } from "node:url";
import { EACH } from "./media-size.ts";

const REPO = "albin02forsberg/margin";

/** Names in ALL that the comma list ONLY selects (all when ONLY is empty). */
export function selected(all: string[], only = ""): string[] {
  const want = only.split(",").map((s) => s.trim()).filter(Boolean);
  return want.length ? all.filter((n) => want.includes(n)) : all;
}

/** Raw URL of FILE (a basename) for pull request / issue N. */
export const mediaUrl = (n: number, file: string, repo = REPO) => `https://raw.githubusercontent.com/${repo}/media/pr-${n}/${file}`;

/** Markdown embedding FILES for N. */
export const markdown = (n: number, files: string[]) => files.map((f) => `![${f.replace(/\.\w+$/, "")}](${mediaUrl(n, f)})`).join("\n\n");

const git = (cwd: string, ...a: string[]) => execFileSync("git", a, { cwd, encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] });

/** Commit FILES to media:pr-N/, creating the branch on first use; a rejected push is rebased and retried. */
export function publish(n: number, files: string[]) {
  for (const f of files) if (statSync(f).size > EACH) throw new Error(`${f} is over the size cap`);
  const wt = mkdtempSync(join(tmpdir(), "margin-media-"));
  try {
    let exists = true;
    try { git(".", "fetch", "origin", "media"); } catch { exists = false; }
    if (exists) git(".", "worktree", "add", "-B", "media", wt, "origin/media");
    else { git(".", "worktree", "add", "--detach", wt); git(wt, "checkout", "--orphan", "media"); git(wt, "rm", "-rfq", "--ignore-unmatch", "."); }
    mkdirSync(join(wt, `pr-${n}`), { recursive: true });
    for (const f of files) copyFileSync(f, join(wt, `pr-${n}`, basename(f)));
    git(wt, "add", ".");
    git(wt, "commit", "-qm", `media for #${n}`);
    for (let i = 0; ; i++) {
      try { git(wt, "push", "-q", "origin", "media"); break; }
      catch (e) { if (i >= 4) throw e; git(wt, "pull", "-q", "--rebase", "origin", "media"); }
    }
  } finally {
    try { git(".", "worktree", "remove", "--force", wt); } catch { rmSync(wt, { recursive: true, force: true }); }
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const [pr, ...rest] = process.argv.slice(2);
  const body = rest.includes("--body");
  const files = rest.filter((a) => a !== "--body");
  if (!pr || !files.length) { console.error("usage: pr-media.ts <pr> <files...> [--body]"); process.exit(2); }
  publish(+pr, files);
  const md = markdown(+pr, files.map((f) => basename(f)));
  console.log(md);
  if (body) {
    const cur = execFileSync("gh", ["pr", "view", pr, "-R", REPO, "--json", "body", "-q", ".body"], { encoding: "utf8" }).replace(/\n## Preview\n[\s\S]*?(?=\n## |\n🤖 Generated|$)/, "");
    const at = cur.lastIndexOf("\n🤖 Generated");
    const next = at < 0 ? `${cur.trimEnd()}\n\n## Preview\n\n${md}\n` : `${cur.slice(0, at).trimEnd()}\n\n## Preview\n\n${md}\n${cur.slice(at)}`;
    execFileSync("gh", ["pr", "edit", pr, "-R", REPO, "--body", next], { stdio: "inherit" });
  }
}
