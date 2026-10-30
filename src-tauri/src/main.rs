// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Before run(): the single-instance plugin would hand `--mcp` to a running app.
    if std::env::args().any(|a| a == "--mcp") {
        return margin_lib::mcp();
    }
    margin_lib::run()
}
