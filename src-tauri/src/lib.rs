mod ipc;

use ncl_core::{AppConfig, PathLayout};
use tokio::sync::RwLock;

/// Tauri 全局应用状态。所有 IPC 命令通过 `State<'_, AppState>` 访问。
pub struct AppState {
    pub layout: PathLayout,
    pub config: RwLock<AppConfig>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            use tauri::Manager;

            let layout = PathLayout::autodetect()?;
            layout.ensure_all()?;
            tracing::info!(
                data_root = %layout.data_root.display(),
                mode = ?layout.mode,
                "NCL path layout resolved"
            );

            let config = AppConfig::load_or_default(&layout.config_file())?;
            tracing::info!(?config, "NCL app config loaded");

            app.manage(AppState {
                layout,
                config: RwLock::new(config),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ipc::core::core_get_paths,
            ipc::core::core_get_config,
            ipc::core::core_set_config,
            ipc::vanilla::vanilla_list_versions,
            ipc::vanilla::vanilla_install,
            ipc::loader::loader_list_versions,
            ipc::loader::loader_install,
            ipc::mods::mod_scan,
            ipc::mods::mod_set_enabled,
            ipc::java::java_scan,
            ipc::launch::launch_run,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(
            "info,nova_craft_launcher=debug,ncl_core=debug,ncl_net=debug,\
             ncl_task=debug,ncl_vanilla=debug,ncl_java=debug,ncl_launch=debug",
        )
    });

    let _ = fmt().with_env_filter(filter).try_init();
}
