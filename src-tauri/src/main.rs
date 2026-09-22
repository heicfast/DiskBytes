//! DiskBytes binary entry (spec §2): starts the Tauri app via
//! `diskbytes_lib::run`.

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    diskbytes_lib::run();
}
