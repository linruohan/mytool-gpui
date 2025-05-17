//TODO: consider generating shader code for WGSL
//TODO: deprecate "runtime-shaders" and "macos-blade"

use std::env;

fn main() {
    let target = env::var("CARGO_CFG_TARGET_OS");
    println!("cargo::rustc-check-cfg=cfg(gles)");

    check_wgsl_shaders();

    match target.as_deref() {
        #[cfg(target_os = "windows")]
        Ok("windows") => {
            let manifest = std::path::Path::new("resources/windows/gpui.manifest.xml");
            let rc_file = std::path::Path::new("resources/windows/gpui.rc");
            println!("cargo:rerun-if-changed={}", manifest.display());
            println!("cargo:rerun-if-changed={}", rc_file.display());
            embed_resource::compile(rc_file, embed_resource::NONE)
                .manifest_required()
                .unwrap();
            #[cfg(target_env = "msvc")]
            {
                // todo(windows): This is to avoid stack overflow. Remove it when solved.
                println!("cargo:rustc-link-arg=/stack:{}", 8 * 1024 * 1024);
            }

            let icon = std::path::Path::new("resources/windows/app-icon.ico");
            println!("cargo:rerun-if-changed={}", icon.display());
            let mut res = winresource::WindowsResource::new();

            // Depending on the security applied to the computer, winresource might fail
            // fetching the RC path. Therefore, we add a way to explicitly specify the
            // toolkit path, allowing winresource to use a valid RC path.
            if let Some(explicit_rc_toolkit_path) = std::env::var("ZED_RC_TOOLKIT_PATH").ok() {
                res.set_toolkit_path(explicit_rc_toolkit_path.as_str());
            }
            res.set_icon(icon.to_str().unwrap());
            res.set("FileDescription", "Zed");
            res.set("ProductName", "Zed");

            if let Err(e) = res.compile() {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        _ => (),
    };
}

#[allow(dead_code)]
fn check_wgsl_shaders() {
    use std::path::PathBuf;
    use std::process;
    use std::str::FromStr;

    let shader_source_path = "./src/platform/blade/shaders.wgsl";
    let shader_path = PathBuf::from_str(shader_source_path).unwrap();
    println!("cargo:rerun-if-changed={}", &shader_path.display());

    let shader_source = std::fs::read_to_string(&shader_path).unwrap();

    match naga::front::wgsl::parse_str(&shader_source) {
        Ok(_) => {
            // All clear
        }
        Err(e) => {
            eprintln!("WGSL shader compilation failed:\n{}", e);
            process::exit(1);
        }
    }
}
