use std::marker::PhantomData;

use tracing::{Event, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

pub trait Suspendable {
    fn suspend<F: FnOnce() -> R, R>(&self, f: F) -> R;
}

/// A [`Layer`] that suspends a suspendable object while logs are being emitted.
///
/// This is useful for preventing logs from being emitted
/// while another renderer is active, such as a spinner.
pub struct SuspendableLayer<S, L, T> {
    inner: L,
    suspendable: T,
    _subscriber: PhantomData<fn(S)>,
}

impl<S, L, T> SuspendableLayer<S, L, T>
where
    S: Subscriber,
    L: Layer<S>,
    T: Suspendable + 'static,
{
    pub fn new(inner: L, suspendable: T) -> Self {
        Self {
            inner,
            suspendable,
            _subscriber: PhantomData,
        }
    }
}

impl<S, L, T> Layer<S> for SuspendableLayer<S, L, T>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    L: Layer<S>,
    T: Suspendable + 'static,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        self.suspendable.suspend(|| {
            self.inner.on_event(event, ctx);
        });
    }
}
