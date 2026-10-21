// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Mapist entry point.
//!
//! One binary, two modes:
//!
//! * `mapist` — opens the desktop window.
//! * `mapist --mcp` — speaks Model Context Protocol on stdin/stdout and never
//!   opens a window. This is what an agent connects to.
//!
//! Keeping both in one executable means the installer ships a single file, and
//! the agent path is guaranteed to run exactly the same core as the GUI.

fn main() {
    // `--mcp` anywhere in the arguments selects protocol mode, so a client can
    // pass it alongside its own flags without this having to parse them.
    if std::env::args().skip(1).any(|a| a == "--mcp") {
        std::process::exit(mapist_mcp::serve());
    }
    mapist_app::run()
}
