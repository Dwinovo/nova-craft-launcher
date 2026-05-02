//! NCL CLI: 阶段 2 才会真正实现。Sprint 0 仅作为占位 binary，
//! 验证业务 crate 与 Tauri 解耦——可以脱离 Tauri 单独编译运行。

use ncl_core::PathLayout;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let layout = PathLayout::autodetect()?;
    println!("Nova Craft Launcher CLI (scaffold)");
    println!("data root : {}", layout.data_root.display());
    println!("mode      : {:?}", layout.mode);
    println!();
    println!("Phase 2 will implement: install / launch / mod list/enable/disable");
    Ok(())
}
