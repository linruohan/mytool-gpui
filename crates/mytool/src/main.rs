#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]
use std::process;

use gpui_component_assets::Assets;
use mytool::{Gallery, init_plugins, todo_state::get_todo_conn};

#[tokio::main]
async fn main() {
    eprintln!("🚀 Starting MyTool-GPUI...");

    let app = gpui_platform::application().with_assets(Assets);

    eprintln!("📦 GPUI application created");

    let name = std::env::args().nth(1);

    // 添加数据库连接错误处理
    eprintln!("🔌 Connecting to database...");
    let db = match get_todo_conn().await {
        Ok(db) => {
            eprintln!("✅ Database connected");
            db
        },
        Err(e) => {
            eprintln!("❌ Failed to connect to database: {:?}", e);
            process::exit(1);
        },
    };

    eprintln!("🎯 Running application...");
    app.run(move |cx| {
        eprintln!("🔧 Initializing...");
        mytool::init(cx);
        eprintln!("📊 Initializing state...");
        // 初始化状态
        mytool::todo_state::state_init(cx, db);
        eprintln!("🔌 Initializing plugins...");
        init_plugins(cx);
        cx.activate(true);
        eprintln!("🪟 Creating window...");
        mytool::create_new_window(
            "MyTool-GPUI",
            move |window, cx| Gallery::view(name.as_deref(), window, cx),
            cx,
        );
        eprintln!("✅ Window created");
    });
}
