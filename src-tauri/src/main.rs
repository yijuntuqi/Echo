// Echo is a library-plus-binary crate: `lib.rs` holds the app so the same code
// can be compiled for desktop and mobile, and this file is the desktop entry.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    echo_lib::run()
}
