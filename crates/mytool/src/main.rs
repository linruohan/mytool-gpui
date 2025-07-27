// Turns off console window on Windows, but not when building with dev profile.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
use gpui::Application;
use mytool::*;
use todos::Store;

#[tokio::main]
async fn main() {
    let app = Application::new().with_assets(Assets);

    // Parse `cargo run -- <story_name>`
    let name = std::env::args().nth(1);
    let db = todo_database_init().await;
    {
        let tmp_db = db.lock().await.clone();
        let projects = Store::new(tmp_db).await.projects().await;
        println!("Loaded projects: {}", projects.len());
    }
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
