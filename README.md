# toolchain
x86_64-pc-windows-msvc
# 
```bash
# .cargo/config.toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "link-arg=/STACK:8000000"]
```