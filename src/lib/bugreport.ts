// The palette's "Report a bug": a prefilled GitHub new-issue URL (no note content or config values, the user reviews it before submitting).

export const ISSUES = "https://github.com/albin02forsberg/margin/issues/new";

export type BugEnv = { version: string; os: string; nightly: boolean; error?: string };

/** The new-issue URL with a report template and the app's environment; ERROR (the last ⚠ message) is capped at 300 chars. */
export function bugReportUrl({ version, os, nightly, error }: BugEnv): string {
  const err = error ? (error.length > 300 ? error.slice(0, 300) + "…" : error) : "none";
  const body = [
    "**What happened**\n\n\n**Steps to reproduce**\n1. \n\n**What you expected**\n\n",
    "**Environment**",
    `- Margin: ${version || "unknown"} (${nightly ? "nightly" : "stable"} updates)`,
    `- OS: ${os || "unknown"}`,
    `- Last error: ${err}`,
  ].join("\n");
  return `${ISSUES}?${new URLSearchParams({ labels: "bug", title: "Bug: ", body })}`;
}

/** Whether a message-line text is an error (⚠-prefixed), i.e. what "Report a bug" shows as the last error. */
export const isError = (m: string) => m.startsWith("⚠");

/** A short OS name from a user agent string (the webview's navigator.userAgent). */
export function osName(ua: string): string {
  const m = /Windows NT [\d.]+|Mac OS X [\d_]+|Android [\d.]+|Linux|CrOS/.exec(ua);
  return m ? m[0].replace(/_/g, ".") : ua.slice(0, 80);
}
