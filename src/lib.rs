use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

/// Task execution stats.
#[derive(Clone, Debug)]
pub struct ExecutionStats {
    /// Task creation timestamp.
    pub created: Instant,

    /// Timestamp of the first poll.
    pub started: Option<Instant>,

    /// Timestamp of the poll that returned `Poll::Ready`.
    pub finished: Option<Instant>,

    /// Total time spent polling.
    pub poll_duration: Duration,

    /// Maximum time spent in the poll method.
    pub poll_duration_max: Duration,

    /// Number of times the task was polled during execution.
    pub poll_entries: usize,
}

/// Trait for tracking task execution stats with [`MetricsFuture`].
pub trait Recorder {
    /// Reports that a task was created.
    fn task_created(&self);

    /// Reports task execution stats when it's dropped.
    fn task_destroyed(&self, stats: ExecutionStats);
}

/// Convenience trait that simplifies the construction of [`MetricsFuture`].
pub trait FutureExt {
    type Future;

    /// Consumes the future, returning a new future that records the execution
    /// stats.
    fn with_metrics<R: Recorder>(self, recorder: R) -> MetricsFuture<Self::Future, R>;
}

struct State<R: Recorder> {
    created: Instant,
    started: Option<Instant>,
    finished: Option<Instant>,
    poll_duration: Duration,
    poll_duration_max: Duration,
    poll_entries: usize,
    recorder: R,
}

impl<R: Recorder> State<R> {
    fn new(recorder: R) -> Self {
        recorder.task_created();

        Self {
            created: Instant::now(),
            started: None,
            finished: None,
            poll_duration: Duration::ZERO,
            poll_duration_max: Duration::ZERO,
            poll_entries: 0,
            recorder,
        }
    }
}

impl<R: Recorder> Drop for State<R> {
    fn drop(&mut self) {
        self.recorder.task_destroyed(ExecutionStats {
            created: self.created,
            started: self.started,
            finished: self.finished,
            poll_duration: self.poll_duration,
            poll_duration_max: self.poll_duration_max,
            poll_entries: self.poll_entries,
        });
    }
}

#[pin_project::pin_project]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct MetricsFuture<F, R: Recorder> {
    #[pin]
    inner: F,
    state: State<R>,
}

impl<F, R: Recorder> MetricsFuture<F, R> {
    pub fn new(inner: F, recorder: R) -> Self {
        Self {
            inner,
            state: State::new(recorder),
        }
    }
}

impl<F, R> Future for MetricsFuture<F, R>
where
    F: Future,
    R: Recorder,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();

        let poll_start = Instant::now();
        let result = this.inner.poll(cx);
        let poll_end = Instant::now();

        let state = this.state;

        if state.started.is_none() {
            state.started = Some(poll_start);
        }

        if result.is_ready() && state.finished.is_none() {
            state.finished = Some(poll_end);
        }

        let poll_duration = poll_end - poll_start;

        state.poll_duration += poll_duration;
        state.poll_duration_max = state.poll_duration_max.max(poll_duration);
        state.poll_entries += 1;

        result
    }
}

impl<T: Future> FutureExt for T {
    type Future = T;

    fn with_metrics<R: Recorder>(self, recorder: R) -> MetricsFuture<Self::Future, R> {
        MetricsFuture::new(self, recorder)
    }
}
