#![allow(clippy::disallowed_methods, reason = "build scripts are exempt")]
#![cfg_attr(not(target_os = "macos"), allow(unused))]

use std::env;

fn main() {
    let target = env::var("CARGO_CFG_TARGET_OS");
    println!("cargo::rustc-check-cfg=cfg(gles)");

    match target.as_deref() {
        Ok("macos") => {
            #[cfg(target_os = "macos")]
            macos::build();
        },
        Ok("windows") => {
            #[cfg(target_os = "windows")]
            windows::build();
        },
        _ => (),
    };
}
#[cfg(target_os = "macos")]
mod macos {
    use std::{
        env,
        path::{Path, PathBuf},
    };

    use cbindgen::Config;

    pub(super) fn build() {}
}

#[cfg(target_os = "windows")]
mod windows {
    use std::{
        ffi::OsString,
        fs,
        io::Write,
        path::{Path, PathBuf},
        process::{self, Command},
    };

    pub(super) fn build() {
        embed_resource();
    }

    fn embed_resource() {
        let rc_file = std::path::Path::new("resources/windows/mytool.rc");
        let manifest = std::path::Path::new("resources/windows/mytool.manifest.xml");
        let icon = std::path::Path::new("resources/windows/app-icon.ico");

        // 追踪资源文件变更
        println!("cargo:rerun-if-changed={}", rc_file.display());
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rerun-if-changed={}", icon.display());

        // 编译 .rc 文件（包含图标和 manifest）
        // 使用 NONE 标志，不启用自动 manifest 处理
        embed_resource::compile(rc_file, embed_resource::NONE);
    }
}
