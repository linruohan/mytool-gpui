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
