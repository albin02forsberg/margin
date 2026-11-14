// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Before run(): the single-instance plugin would hand `--mcp` to a running app.
    if std::env::args().any(|a| a == "--mcp") {
        return margin_lib::mcp();
    }
    // One-shot commands (--add, --status, …) print a line and exit, likewise without the app.
    if let Some(code) = margin_lib::cli(&std::env::args().collect::<Vec<_>>()) {
        std::process::exit(code);
    }
    margin_lib::run()
}
