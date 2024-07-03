use std::future::Future;
use std::pin::Pin;
use dashmap::DashMap;

use crate::{SSSD}; // Replace with your actual crate items

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use tokio::{
    sync::{
        mpsc::{self, error::SendError, Sender},
    },
};

use dashmap::DashMap;

/// Marker-trait for events
pub trait Event: Copy + Clone + SSSD {} // Replace SSSD with your actual trait

impl<S> Event for S where S: Copy + Clone + SSSD {}

/// `EventCallback` is a marker-trait for callback which will be stored in subscribers
/// Auto-implemented
pub trait EventCallback<E>: Send + Sync + 'static {
    fn call<'a>(&'a self, event: E) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>
        where
            Self: 'a;
}

impl<F, E, Fut> EventCallback<E> for F
where
    F: Fn(E) -> Fut + Send + Sync + 'static,
    E: Event,
    Fut: Future<Output = ()> + Send + 'static,
{
    fn call<'a>(&'a self, event: E) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>
        where
            Self: 'a,
    {
        Box::pin(async move {
            self(event).await;
        })
    }
}

/// # Event Bus
///
/// `EventBus<E>`, where `E` implements `Event`
///
/// `EventBus` accessible from all actors
///
/// on `new()` Event Bus spawning background task with loop
/// which is checking for events
///
/// ## Fields
///
/// * `subscribers` - DashMap that contains id of subscriber and tuple of (Event, Callback) which is executed on event (Variant of `E`)
///
/// * `tx`      - Sender channel for publishing events
///
/// * `counter` - Atomic counter for generating subscriber IDs
///
/// ## Methods
///
/// * `new` - Creating new instance of `EventBus`
///
/// * `publish` - Sends an event to all subscribers
///
/// * `subscribe` - Registers a callback for a specific event type
///
/// * `unsubscribe` - Removes a subscriber based on subscriber ID
///
pub struct EventBus<E>
where
    E: Event,
{
    subscribers: Arc<DashMap<usize, Pin<Box<dyn EventCallback<E>>>>>,
    tx: Sender<E>,
    counter: AtomicUsize,
}

impl<E> EventBus<E>
where
    E: Event,
{
    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::channel(1000);

        let event_bus = Self {
            subscribers: Arc::new(DashMap::new()),
            tx: tx,
            counter: AtomicUsize::new(0),
        };

        log::trace!("Event bus is running");
        let subs = event_bus.subscribers.clone();

        let handle = tokio::runtime::Handle::current();
        {
            let _join_handler = handle.spawn(async move {
                while let Some(event) = rx.recv().await {
                    let subscribers = subs.iter();
                    log::trace!("processing event: {:?}", event);
                    for (_id, callback) in subscribers {
                        callback.call(event).await;
                    }
                }
            });
        } // Dropped to make it background task

        event_bus
    }

    pub async fn publish(&self, event: E) -> Result<(), SendError<E>> {
        self.tx.send(event).await?;
        log::trace!("New event in event bus: {:?}", event);
        Ok(())
    }

    pub async fn subscribe<F>(&self, callback: F) -> usize
    where
        F: EventCallback<E>,
    {
        let id = self.counter.fetch_add(1, Ordering::SeqCst);
        log::trace!("New subscriber in event bus. id: {}", id);

        self.subscribers.insert(id, Box::pin(callback));

        id
    }

    pub async fn unsubscribe(&self, subscriber_id: usize) {
        log::trace!("Removed subscriber in event bus. id: {}", subscriber_id);

        self.subscribers.remove(&subscriber_id);
    }
}
// Revie this code 