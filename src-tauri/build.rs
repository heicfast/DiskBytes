//! Tauri build script: generates the `WebView` context + Windows
//! resources (icons, manifest) declared in `tauri.conf.json` and
//! `capabilities/`.

fn main() {
    tauri_build::build();
}
