// Turns off console window on Windows, but not when building with dev profile.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
#![allow(dead_code)]
use gpui::*;
use mytool::*;
mod gallery;

use gallery::Gallery;

fn main() {
    let app = Application::new().with_assets(Assets);
    app.run(move |cx: &mut App| {
        mytool::init(cx);
        cx.activate(true);
        mytool::create_new_window("MyTool-GPUI", Gallery::view, cx);
    });
}
