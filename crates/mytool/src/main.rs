#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]
use std::process;

use gpui_component_assets::Assets;
use mytool::{MainView, init_plugins, todo_state::get_todo_conn};

#[tokio::main]
async fn main() {
    let app = gpui_platform::application().with_assets(Assets);

    let name = std::env::args().nth(1);

    // 添加数据库连接错误处理
    let db = match get_todo_conn().await {
        Ok(db) => db,
        Err(e) => {
            tracing::error!("Failed to connect to database: {:?}", e);
            process::exit(1);
        },
    };

    app.run(move |cx| {
        mytool::init(cx);
        // 初始化状态
        mytool::todo_state::state_init(cx, db);
        init_plugins(cx);
        cx.activate(true);
        mytool::create_new_window(
            "MyTool-GPUI",
            move |window, cx| MainView::view(name.as_deref(), window, cx),
            cx,
        );
    });
}
