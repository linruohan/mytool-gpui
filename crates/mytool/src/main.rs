// Turns off console window on Windows, but not when building with dev profile.
// 临时注释掉这行,以便在 release 模式下也能看到控制台日志输出
// #![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]
use gpui::Application;
use gpui_component_assets::Assets;
use mytool::{
    MainView, init_plugins,
    todo_state::{get_todo_conn, state_init},
};

#[tokio::main]
async fn main() {
    let app = Application::new().with_assets(Assets);

    let name = std::env::args().nth(1);
    let db = get_todo_conn().await;

    app.run(move |cx| {
        mytool::init(cx);
        state_init(cx, db);
        init_plugins(cx);
        cx.activate(true);
        mytool::create_new_window(
            "MyTool-GPUI",
            move |window, cx| MainView::view(name.as_deref(), window, cx),
            cx,
        );
    });
}
