#![allow(dead_code)]

use gpui::Application;
// use ipc::{client::client_connect, server::setup_socket};

mod assets;
// mod commands;
// mod components;
mod date;
mod db;
// mod hotkey;
// mod ipc;
mod paths;
// mod state;
// mod theme;
// mod window;
mod workspace;
// mod loader;
// mod query;
use assets::Assets;
use gpui::*;
// use theme::Theme;
// use window::{Window, WindowStyle};
use workspace::Workspace;
// #[async_std::main]
fn main() {
    env_logger::init();

    // if let Ok(listener) = setup_socket().await {
    let app = Application::new();
    app.with_assets(Assets).run(move |cx: &mut App| {
        let bounds1 = Bounds::centered(None, size(px(500.), px(500.0)), cx);
        let bounds = Bounds {
            origin: Point::new(Pixels::from(0.0), Pixels::from(0.0)),
            size: Size {
                width: Pixels::from(1920.0),
                height: Pixels::from(1080.0),
            },
        };
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds1)),
            ..Default::default()
        };

        cx.open_window(options, |window: &mut Window, cx| {
            // let theme = cx.global::<Theme>();
            // window.set_background_appearance(WindowBackgroundAppearance::from(
            //     theme.window_background.clone().unwrap_or_default(),
            // ));
            cx.new(|_| Workspace {
                text: "My World".into(),
            })
        })
        .unwrap();
        cx.activate(true);
    });
    // } else if let Err(e) = client_connect().await {
    //     log::error!("CLI Error: {:?}", e);
    // }
}
