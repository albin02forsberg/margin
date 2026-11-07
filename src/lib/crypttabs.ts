/** Tab keys of decrypted-heading reports (`crypt:<path>:<line>`, see cryptView): all of them, or those of note PATH. */
export const isCryptKey = (key: string, path?: string) => key.startsWith(path == null ? "crypt:" : `crypt:${path}:`);

/** How saving tab T goes (#1154): a decrypted tab is only re-locked on an explicit save (FORCE: :w, Ctrl+S),
 *  never by autosave, a tab switch or :wa; everything else takes the normal file path. */
export const saveRoute = (t: { crypt?: unknown }, force: boolean) => (!t.crypt ? "file" : force ? "relock" : "none");
