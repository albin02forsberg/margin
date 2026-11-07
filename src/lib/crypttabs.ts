/** Tab keys of decrypted-heading reports (`crypt:<path>:<line>`, see cryptView): all of them, or those of note PATH. */
export const isCryptKey = (key: string, path?: string) => key.startsWith(path == null ? "crypt:" : `crypt:${path}:`);
