// Windows 发布构建使用 windows 子系统：双击启动时不弹出黑色控制台窗口
// 仅在 release 生效；debug 构建（debug_assertions）不受影响，仍可在终端查看日志
// 如需临时排查发布版日志，可将下面的 "windows" 临时改回 "console"
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]
use std::{process, sync::mpsc::channel, thread};

// 使用 AllAssets：嵌入完整 Lucide 图标目录，gpui_kit::assets::IconName 中的图标才能被加载
use gpui_kit::assets::AllAssets;
use mytool::{Gallery, todo_state::get_todo_conn};

#[tokio::main]
async fn main() {
    let app = gpui_platform::application().with_assets(AllAssets);
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
