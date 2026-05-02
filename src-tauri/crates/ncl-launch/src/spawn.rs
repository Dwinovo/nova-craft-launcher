//! 启动 java 进程并把 stdout/stderr 转发到 [`ProgressSink`]。

use crate::model::LaunchPlan;
use ncl_core::progress::{LogLevel, ProgressEvent, ProgressSink};
use ncl_core::{Error, Result};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

/// 启动后返回的进程句柄。可调 `wait`（等退出码）或 `kill`（强杀）。
pub struct ProcessHandle {
    pub pid: u32,
    child: Child,
}

/// 启动一个游戏进程。stdout / stderr 行被逐行转发到 sink 作为 [`ProgressEvent::Log`]。
pub async fn spawn(plan: LaunchPlan, sink: Arc<dyn ProgressSink>) -> Result<ProcessHandle> {
    let mut cmd = Command::new(&plan.java_path);
    cmd.args(&plan.jvm_args);
    cmd.arg(&plan.main_class);
    cmd.args(&plan.game_args);
    cmd.current_dir(&plan.working_dir);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    // Windows: 不要弹一个额外的控制台窗口（tokio::process::Command 已内置 creation_flags 方法）
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    tracing::info!(
        java = %plan.java_path.display(),
        main = %plan.main_class,
        cwd = %plan.working_dir.display(),
        "spawning Minecraft process"
    );

    let mut child = cmd.spawn().map_err(|e| Error::io(&plan.java_path, e))?;
    let pid = child.id().unwrap_or(0);

    if let Some(stdout) = child.stdout.take() {
        let sink2 = sink.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                sink2
                    .emit(ProgressEvent::Log {
                        source: "minecraft.stdout".to_string(),
                        level: LogLevel::Info,
                        message: line,
                    })
                    .await;
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let sink2 = sink.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                sink2
                    .emit(ProgressEvent::Log {
                        source: "minecraft.stderr".to_string(),
                        level: LogLevel::Warn,
                        message: line,
                    })
                    .await;
            }
        });
    }

    Ok(ProcessHandle { pid, child })
}

impl ProcessHandle {
    /// 等待进程退出，返回退出码（None 表示被信号终止）。
    pub async fn wait(mut self) -> Result<Option<i32>> {
        let status = self
            .child
            .wait()
            .await
            .map_err(|e| Error::Task(format!("wait child: {e}")))?;
        Ok(status.code())
    }

    /// 强制终止。
    pub async fn kill(mut self) -> Result<()> {
        self.child
            .kill()
            .await
            .map_err(|e| Error::Task(format!("kill: {e}")))
    }
}
