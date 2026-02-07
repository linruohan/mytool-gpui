use std::env;

fn main() {
    let target = env::var("CARGO_CFG_TARGET_OS");
    println!("cargo::rustc-check-cfg=cfg(glues)");

    match target.as_deref() {
        #[cfg(target_os = "windows")]
        Ok("windows") => {
            let manifest = std::path::Path::new("resources/windows/mytool.manifest.xml");
            let rc_file = std::path::Path::new("resources/windows/mytool.rc");
            println!("cargo:rerun-if-changed={}", manifest.display());
            println!("cargo:rerun-if-changed={}", rc_file.display());
            if let Err(e) =
                embed_resource::compile(rc_file, embed_resource::NONE).manifest_required()
            {
                eprintln!("embed_resource compile failed: {}", e);
                std::process::exit(1);
            }
            #[cfg(target_env = "msvc")]
            {
                // todo(windows): This is to avoid stack overflow. Remove it when solved.
                println!("cargo:rustc-link-arg=/stack:{}", 8 * 1024 * 1024);
            }

            let icon = std::path::Path::new("resources/windows/app-icon.ico");
            println!("cargo:rerun-if-changed={}", icon.display());
            let mut res = winresource::WindowsResource::new();

            if let Some(icon_str) = icon.to_str() {
                res.set_icon(icon_str);
            } else {
                eprintln!("icon path is not valid unicode: {}", icon.display());
            }
            res.set("FileDescription", "MyTool");
            res.set("ProductName", "MyTool");

            if let Err(e) = res.compile() {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        },
        _ => (),
    };
}
