# tool

Org-mode style notes, agenda and TODOs, Zettelkasten links, vim keys and a
port of my Emacs timeclock — as a Tauri desktop app for Linux, macOS and Windows.

```bash
npm install
npm run tauri dev      # run
npm run tauri build    # release bundle for the current OS
cd src-tauri && cargo test
```

Config lives in the OS config dir (`~/.config/dev.albin.tool/config.toml` on
Linux); open it with `SPC f c`. Notes default to `~/notes`, timeclock data to
`~/timeclock/<profile>/`. Import old Emacs data with `SPC t M`.

## Keys

`SPC` opens the leader menu (which-key style) in normal mode and in the agenda.

| Keys | Action |
|---|---|
| `SPC SPC` / `SPC f f` | find file |
| `SPC /` | full-text search |
| `SPC a a` / `SPC a t` | week agenda / TODO list |
| `SPC n f` / `n i` / `n n` | find node / insert `[[id:]]` link / new note |
| `SPC n b` / `n l` | backlinks panel / jump to backlink |
| `SPC m t s d x` | cycle TODO, schedule, deadline, checkbox |
| `SPC t …` | timeclock (same letters as the old `C-c t` transient) |
| `Tab` / `S-Tab` | fold heading / fold all |
| `M-h` `M-l` `M-j` `M-k` | promote, demote, move subtree |
| `M-Enter` | new heading |
| `S-←` `S-→` | cycle TODO keyword |
| `Enter` / `gf` | follow link |
| `gt` / `gT`, `C-Tab` | next / previous tab |
| `:w` `:q` `:wq` | save / close tab |

Agenda: `j/k`, `Enter`, `t/T` cycle, `s` schedule, `d` deadline, `I/O` clock
in/out, `f/b` week, `.` today, `/` filter (TODO list), `q` close.

Dates accept org-read-date style input: `today`, `+3d`, `-1w`, `fri`,
`12-24`, `2026-12-24 14:00`, `rm` (remove).
