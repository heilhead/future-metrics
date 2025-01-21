use future_metrics::{ExecutionStats, FutureExt as _, Recorder};

struct MyRecorder;

impl Recorder for MyRecorder {
    fn task_created(&self) {
        // Future was created.
    }

    fn task_destroyed(&self, stats: ExecutionStats) {
        // Future was destroyed.

        println!("{stats:?}");

        // Record execution stats with your metrics backend of choice.
    }
}

#[tokio::main]
async fn main() {
    tokio::time::sleep(std::time::Duration::from_millis(300))
        .with_metrics(MyRecorder)
        .await;
}
