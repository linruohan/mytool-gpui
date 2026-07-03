// 🚀 临时禁用 windows_subsystem 以显示控制台日志（调试用）
// 正式发布时请取消下面这行的注释，并注释掉 console 那行
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "console")]
// #![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]
use std::{process, sync::mpsc::channel, thread};

use gpui_component_assets::Assets;
use mytool::{Gallery, todo_state::get_todo_conn};

#[tokio::main]
async fn main() {
    let app = gpui_platform::application().with_assets(Assets);
    let name = std::env::args().nth(1);

    let db = match get_todo_conn().await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("❌ 数据库连接失败: {:?}", e);
            process::exit(1);
        },
    };

    // 🚀 创建退出信号通道
    let (tx, rx) = channel::<bool>();

    // 初始化 lib.rs 中的 SHUTDOWN_SENDER
    unsafe {
        mytool::SHUTDOWN_SENDER = Some(tx);
    }

    // 🚀 启动后台退出监控线程
    let exit_handle = thread::spawn(move || {
        match rx.recv() {
            Ok(_) | Err(_) => {},
        }
        eprintln!("✅ 收到退出信号，进程即将退出");
        thread::sleep(std::time::Duration::from_millis(100));
        process::exit(0);
    });

    app.run(move |cx| {
        mytool::init(cx);
        mytool::todo_state::state_init(cx, db);
        cx.activate(true);
        mytool::create_new_window(
            "MyTool-GPUI",
            move |window, cx| Gallery::view(name.as_deref(), window, cx),
            cx,
        );
    });

    // app.run() 返回后（正常退出路径）
    eprintln!("✅ app.run() 已返回");
    mytool::request_shutdown();
    let _ = exit_handle.join();
}
