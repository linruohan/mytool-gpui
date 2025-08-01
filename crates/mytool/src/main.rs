// Turns off console window on Windows, but not when building with dev profile.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
use gpui::Application;
use mytool::*;

#[tokio::main]
async fn main() {
    let app = Application::new().with_assets(Assets);

    let name = std::env::args().nth(1);
    let db = todo_database_init().await;

    app.run(move |cx| {
        mytool::init(cx);
        cx.set_global(DBState { conn: db });
        cx.activate(true);
        mytool::create_new_window(
            "MyTool-GPUI",
            move |window, cx| Gallery::view(name.as_deref(), window, cx),
            cx,
        );
    });
}
