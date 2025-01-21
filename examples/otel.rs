use {
    future_metrics::{ExecutionStats, FutureExt as _, Recorder},
    opentelemetry::metrics::{Counter, Histogram, Meter, MeterProvider},
    opentelemetry_sdk::metrics::SdkMeterProvider,
    prometheus::{Encoder, TextEncoder},
    rand::Rng as _,
    std::{sync::Arc, time::Duration},
};

struct Inner {
    tasks_created: Counter<u64>,
    tasks_finished: Counter<u64>,
    tasks_cancelled: Counter<u64>,
    total_duration: Histogram<f64>,
    poll_duration: Histogram<f64>,
}

#[derive(Clone)]
struct StatsRecorder {
    inner: Arc<Inner>,
}

impl StatsRecorder {
    fn new(meter: &Meter, prefix: &str) -> Self {
        let inner = Arc::new(Inner {
            tasks_created: meter.u64_counter(format!("{prefix}_created")).build(),
            tasks_finished: meter.u64_counter(format!("{prefix}_finished")).build(),
            tasks_cancelled: meter.u64_counter(format!("{prefix}_cancelled")).build(),
            total_duration: meter
                .f64_histogram(format!("{prefix}_duration"))
                .with_boundaries(vec![0.1, 0.2, 0.3])
                .build(),
            poll_duration: meter
                .f64_histogram(format!("{prefix}_poll_duration"))
                .with_boundaries(vec![])
                .build(),
        });

        Self { inner }
    }
}

impl Recorder for StatsRecorder {
    fn task_created(&self) {
        self.inner.tasks_created.add(1, &[]);
    }

    fn task_destroyed(&self, stats: ExecutionStats) {
        if let (Some(started), Some(finished)) = (stats.started, stats.finished) {
            let total_duration = finished - started;

            self.inner.tasks_finished.add(1, &[]);
            self.inner
                .total_duration
                .record(total_duration.as_secs_f64(), &[]);
            self.inner
                .poll_duration
                .record(stats.poll_duration.as_secs_f64(), &[]);
        } else {
            self.inner.tasks_cancelled.add(1, &[]);
        }
    }
}

async fn my_task(dur: Duration) {
    tokio::time::sleep(dur).await;
}

#[tokio::main]
async fn main() {
    // Create a new prometheus registry.
    let registry = prometheus::Registry::new();

    // Configure OpenTelemetry to use this registry.
    let exporter = opentelemetry_prometheus::exporter()
        .with_registry(registry.clone())
        .build()
        .unwrap();

    // Set up a meter to create instruments.
    let provider = SdkMeterProvider::builder().with_reader(exporter).build();
    let meter = provider.meter("my-app");

    // Create task stats recorder.
    let recorder = StatsRecorder::new(&meter, "my_task");

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
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    let mut result = Vec::new();
    encoder.encode(&metric_families, &mut result).unwrap();

    println!("{}", String::from_utf8(result).unwrap());
}
