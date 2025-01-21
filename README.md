# future-metrics

Instrument futures with execution metrics.

The primary use case is keeping track of the async tasks created and detached, as well as monitoring their execution to identify potential bottlenecks or async executor blocking.

## Usage

Adding dependency:

```toml
[dependencies]
future-metrics = "0.1"
```

The very basic example of collecting execution stats from an async task:

```rust
use future_metrics::{ExecutionStats, FutureExt as _, Recorder};

struct MyRecorder;

impl Recorder for MyRecorder {
    fn task_created(&self) {
        // Future was created.
    }

    fn task_destroyed(&self, stats: ExecutionStats) {
        // Future was destroyed.
        println!("{stats:?}");
    }
}

#[tokio::main]
async fn main() {
    tokio::time::sleep(std::time::Duration::from_millis(300))
        .with_metrics(MyRecorder)
        .await;
}
```

The above would output something like this:

```
ExecutionStats { created: Instant { tv_sec: 9919, tv_nsec: 707589973 }, started: Some(Instant { tv_sec: 9919, tv_nsec: 707590083 }), finished: Some(Instant { tv_sec: 9920, tv_nsec: 9048028 }), poll_duration: 13.52µs, poll_duration_max: 9.48µs, poll_entries: 2 }
```

Which can be integrated with any metrics backend.

More examples can be found in `examples/` directory.

# License

[Apache 2.0](LICENSE)
