#![allow(dead_code)]
use gpui::*;

mod date;
mod db;
mod paths;
mod workspace;
use workspace::open_new;

use mytool::{Assets, Open, Quit};

#[cfg(debug_assertions)]
const STATE_FILE: &str = "target/layout.json";
#[cfg(not(debug_assertions))]
const STATE_FILE: &str = "layout.json";

pub fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);

    cx.on_action(|_action: &Open, _cx: &mut App| {});

    gpui_component::init(cx);
    mytool::init(cx);
}

fn main() {
    use gpui_component::input::{Copy, Cut, Paste, Redo, Undo};
    env_logger::init();
    let app = Application::new().with_assets(Assets);
    app.run(move |cx: &mut App| {
        init(cx);
        cx.on_action(quit);
        cx.set_menus(vec![
            Menu {
                name: "GPUI App".into(),
                items: vec![MenuItem::action("Quit", Quit)],
            },
            Menu {
                name: "Edit".into(),
                items: vec![
                    MenuItem::os_action("Undo", Undo, gpui::OsAction::Undo),
                    MenuItem::os_action("Redo", Redo, gpui::OsAction::Redo),
                    MenuItem::separator(),
                    MenuItem::os_action("Cut", Cut, gpui::OsAction::Cut),
                    MenuItem::os_action("Copy", Copy, gpui::OsAction::Copy),
                    MenuItem::os_action("Paste", Paste, gpui::OsAction::Paste),
                ],
            },
            Menu {
                name: "Window".into(),
                items: vec![],
            },
        ]);
        cx.activate(true);
        open_new(cx, |_, _, _| {
            // do something
        })
        .detach();
    });
}
fn quit(_: &Quit, cx: &mut App) {
    cx.quit();
}
