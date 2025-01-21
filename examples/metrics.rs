use {
    future_metrics::{ExecutionStats, FutureExt as _, Recorder},
    metrics_exporter_prometheus::PrometheusBuilder,
    rand::Rng as _,
    std::time::Duration,
};

#[derive(Clone)]
struct StatsRecorder {
    task_name: &'static str,
}

impl StatsRecorder {
    fn new(task_name: &'static str) -> Self {
        Self { task_name }
    }
}

impl Recorder for StatsRecorder {
    fn task_created(&self) {
        metrics::counter!("tasks_created", "task_name" => self.task_name).increment(1);
    }

    fn task_destroyed(&self, stats: ExecutionStats) {
        let labels = [("task_name", self.task_name)];

        if let (Some(started), Some(finished)) = (stats.started, stats.finished) {
            let total_duration = finished - started;

            metrics::counter!("tasks_finished", &labels).increment(1);
            metrics::histogram!("tasks_total_duration", &labels)
                .record(total_duration.as_secs_f64());
            metrics::histogram!("tasks_poll_duration", &labels)
                .record(stats.poll_duration.as_secs_f64());
        } else {
            metrics::counter!("tasks_cancelled", &labels).increment(1);
        }
    }
}

async fn my_task(dur: Duration) {
    tokio::time::sleep(dur).await;
}

#[tokio::main]
async fn main() {
    // Initialize prometheus.
    let handle = PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install recorder");

    // Create task stats recorder.
    let recorder = StatsRecorder::new("my_task");

    // Spawn a bunch of tasks.
    for _ in 0..100 {
        let dur = Duration::from_millis(rand::thread_rng().gen_range(100..300));

        tokio::spawn(my_task(dur).with_metrics(recorder.clone()));
    }

    // Record a cancelled task.
    drop(my_task(Duration::ZERO).with_metrics(recorder.clone()));

    // Wait for async work to finish.
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Export prometheus data.
    println!("{}", handle.render());
}
