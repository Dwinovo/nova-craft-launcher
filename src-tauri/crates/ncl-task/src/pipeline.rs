use crate::task::BoxTask;
use ncl_core::progress::{LogLevel, ProgressEvent, ProgressSink};
use ncl_core::{Error, Result};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use uuid::Uuid;

/// 一组可并行执行的任务。同一 stage 内的 task 互不依赖；
/// 不同 stage 之间严格串行。
pub struct Stage {
    pub label: String,
    pub tasks: Vec<BoxTask>,
}

impl Stage {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            tasks: Vec::new(),
        }
    }

    pub fn push(mut self, task: BoxTask) -> Self {
        self.tasks.push(task);
        self
    }

    pub fn extend(mut self, tasks: impl IntoIterator<Item = BoxTask>) -> Self {
        self.tasks.extend(tasks);
        self
    }

    pub fn total_weight(&self) -> u64 {
        self.tasks.iter().map(|t| t.weight()).sum()
    }
}

/// 多阶段流水线。
pub struct Pipeline {
    pub label: String,
    pub stages: Vec<Stage>,
    pub concurrency: usize,
}

impl Pipeline {
    pub fn new(label: impl Into<String>, concurrency: usize) -> Self {
        Self {
            label: label.into(),
            stages: Vec::new(),
            concurrency: concurrency.max(1),
        }
    }

    pub fn add_stage(mut self, stage: Stage) -> Self {
        self.stages.push(stage);
        self
    }

    pub fn total_weight(&self) -> u64 {
        self.stages.iter().map(|s| s.total_weight()).sum()
    }

    /// 顺序执行所有 stage。每个 stage 内的 task 用全局 `Semaphore` 控并发并行。
    /// 任何任务失败 → 等待当前 stage 内的剩余任务完成（避免悬挂）但跳过后续 stage，
    /// 然后返回首个 Err。
    pub async fn execute(self, sink: Arc<dyn ProgressSink>) -> Result<()> {
        let total = self.total_weight();
        let mut completed = 0u64;
        let pipeline_id = Uuid::new_v4().to_string();

        sink.emit(ProgressEvent::TaskStarted {
            task_id: pipeline_id.clone(),
            label: self.label.clone(),
            total_weight: total,
        })
        .await;

        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let mut first_err: Option<Error> = None;

        'outer: for stage in self.stages {
            sink.emit(ProgressEvent::Log {
                source: "pipeline".to_string(),
                level: LogLevel::Info,
                message: format!("→ {}", stage.label),
            })
            .await;

            let mut joinset: JoinSet<(Result<()>, u64, String)> = JoinSet::new();
            for task in stage.tasks {
                let sem = semaphore.clone();
                let weight = task.weight();
                let label = task.label().to_string();
                joinset.spawn(async move {
                    let _permit = match sem.acquire().await {
                        Ok(p) => p,
                        Err(_) => return (Err(Error::Task("semaphore closed".into())), weight, label),
                    };
                    let result = task.run().await;
                    (result, weight, label)
                });
            }

            while let Some(joined) = joinset.join_next().await {
                let (result, weight, label) = match joined {
                    Ok(v) => v,
                    Err(je) => (
                        Err(Error::Task(format!("task panicked: {je}"))),
                        0,
                        "<panicked>".to_string(),
                    ),
                };
                match result {
                    Ok(()) => {
                        completed += weight;
                        sink.emit(ProgressEvent::TaskProgress {
                            task_id: pipeline_id.clone(),
                            completed,
                            total,
                            message: Some(label),
                        })
                        .await;
                    }
                    Err(e) => {
                        sink.emit(ProgressEvent::Log {
                            source: "pipeline".to_string(),
                            level: LogLevel::Error,
                            message: format!("task '{label}' failed: {e}"),
                        })
                        .await;
                        if first_err.is_none() {
                            first_err = Some(e);
                        }
                    }
                }
            }

            if first_err.is_some() {
                break 'outer;
            }
        }

        let success = first_err.is_none();
        let err_msg = first_err.as_ref().map(|e| e.to_string());
        sink.emit(ProgressEvent::TaskFinished {
            task_id: pipeline_id.clone(),
            success,
            error: err_msg,
        })
        .await;

        first_err.map_or(Ok(()), Err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use ncl_core::progress::{ChannelSink, ProgressEvent};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    struct CountingTask {
        label: String,
        weight: u64,
        counter: Arc<AtomicUsize>,
        delay_ms: u64,
        fail: bool,
    }

    #[async_trait]
    impl crate::Task for CountingTask {
        fn label(&self) -> &str {
            &self.label
        }
        fn weight(&self) -> u64 {
            self.weight
        }
        async fn run(&self) -> Result<()> {
            tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
            self.counter.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                Err(Error::Task(format!("intentional fail: {}", self.label)))
            } else {
                Ok(())
            }
        }
    }

    fn make_task(label: &str, counter: &Arc<AtomicUsize>, delay_ms: u64, fail: bool) -> BoxTask {
        Arc::new(CountingTask {
            label: label.to_string(),
            weight: 10,
            counter: counter.clone(),
            delay_ms,
            fail,
        })
    }

    fn collect_events(mut rx: tokio::sync::mpsc::UnboundedReceiver<ProgressEvent>) -> Vec<ProgressEvent> {
        let mut out = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            out.push(ev);
        }
        out
    }

    #[tokio::test]
    async fn runs_all_tasks_in_topological_order() {
        let counter = Arc::new(AtomicUsize::new(0));
        let (sink, rx) = ChannelSink::new();
        let sink: Arc<dyn ProgressSink> = Arc::new(sink);

        let pipeline = Pipeline::new("test", 4)
            .add_stage(
                Stage::new("stage 1")
                    .push(make_task("a", &counter, 10, false))
                    .push(make_task("b", &counter, 10, false)),
            )
            .add_stage(Stage::new("stage 2").push(make_task("c", &counter, 5, false)));

        pipeline.execute(sink).await.expect("pipeline ok");
        assert_eq!(counter.load(Ordering::SeqCst), 3);

        let events = collect_events(rx);
        assert!(matches!(events.first(), Some(ProgressEvent::TaskStarted { .. })));
        assert!(matches!(events.last(), Some(ProgressEvent::TaskFinished { success: true, .. })));
        // total = 30, completed events should reach 30
        let last_progress = events
            .iter()
            .rev()
            .find_map(|e| match e {
                ProgressEvent::TaskProgress { completed, total, .. } => Some((*completed, *total)),
                _ => None,
            })
            .expect("has progress");
        assert_eq!(last_progress, (30, 30));
    }

    #[tokio::test]
    async fn first_failure_stops_subsequent_stages() {
        let counter = Arc::new(AtomicUsize::new(0));
        let (sink, _rx) = ChannelSink::new();
        let sink: Arc<dyn ProgressSink> = Arc::new(sink);

        let pipeline = Pipeline::new("test", 4)
            .add_stage(
                Stage::new("stage 1")
                    .push(make_task("a", &counter, 1, true))
                    .push(make_task("b", &counter, 1, false)),
            )
            .add_stage(Stage::new("stage 2").push(make_task("c", &counter, 1, false)));

        let res = pipeline.execute(sink).await;
        assert!(res.is_err(), "pipeline should fail");
        // stage 1 内的 a + b 都跑了，但 stage 2 的 c 不能跑
        let runs = counter.load(Ordering::SeqCst);
        assert!(runs == 2, "expected stage 1 fully drained, stage 2 skipped; got {runs}");
    }

    #[tokio::test]
    async fn concurrency_limit_respected() {
        // 8 个 task 每个 sleep 50ms，concurrency=2 → 总耗时 ~ 4×50 = 200ms 起
        let counter = Arc::new(AtomicUsize::new(0));
        let (sink, _rx) = ChannelSink::new();
        let sink: Arc<dyn ProgressSink> = Arc::new(sink);

        let mut stage = Stage::new("parallel");
        for i in 0..8 {
            stage = stage.push(make_task(&format!("t{i}"), &counter, 50, false));
        }
        let pipeline = Pipeline::new("test", 2).add_stage(stage);

        let start = std::time::Instant::now();
        pipeline.execute(sink).await.unwrap();
        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(150),
            "concurrency limit appears violated, elapsed={elapsed:?}"
        );
        assert_eq!(counter.load(Ordering::SeqCst), 8);
    }
}
