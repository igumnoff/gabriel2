use std::future::Future;
use std::pin::Pin;

use crate::{SSSD};

use std::collections::HashMap;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use tokio::{
    sync::{
        mpsc::{
            self,
            error::{SendError},
            Sender
        },
        Mutex
    }
};



/// Marker-trait for events
pub trait Event: Copy + Clone + SSSD {}

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
/// `EventBus<E,R>`, where `E` implements `Event` marker trait `R` `implements EventCallback<E>`
///
/// `EventBus` accessible from all actors
///
/// on `new()` Event Bus spawnin background task with loop
/// which is checking for events
///
/// ## Fields
///
/// * `subscribers` - HashMap that contains id of subsriber and tuple of (Event, Callback) which is executes on event (Variant of `E`)
///
/// * `events`      - Vector of all events
///
/// ## Methods
///
/// * `new` - Creating new instance of `EventBus`
///
/// * `publish` - Sends an event to a
///
/// * `subscribe` - Creates record in `subscribers` of enum variant and callback to that variant, returns subscriber's id
///                 and unwinding event stack, it's promissing that subscribers will react on events which are published
///                 after subscription only
///
/// * `unsubscribe` - Remove record from `subscribers`
///
pub struct EventBus<E>
where
    E: Event,
{
    subscribers: Arc<Mutex<HashMap<usize, Pin<Box<dyn EventCallback<E>>>>>>,
    tx: Sender<E>,
    counter: AtomicUsize,
}

impl<E> EventBus<E>
where
    E: Event,
{
    // FIXME: Calls function only once for some reason...
    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::channel(1000);

        let event_bus = Self {
            subscribers: Arc::new(Mutex::new(HashMap::new())),
            tx: tx,
            counter: AtomicUsize::new(0),
        };

        log::trace!("Event bus is running");
        let subs = event_bus.subscribers.clone();

        let handle = tokio::runtime::Handle::current();
        {
            let _join_handler = handle.spawn(async move {
                loop {
                    while let Some(event) = rx.recv().await {
                        let subscribers = subs.lock().await;
                        log::trace!("processing event: {:?}", event);
                        for (_id, callback) in &*subscribers {
                            callback.call(event).await;
                        }
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

        let mut subscribers = self.subscribers.lock().await;
        subscribers.insert(id, Box::pin(callback));
        id
    }

    pub async fn unsubscribe(&self, subscriber_id: usize) {

        log::trace!("Removed subscriber in event bus. id: {}", subscriber_id);

        let mut subscribers = self.subscribers.lock().await;
        subscribers.remove(&subscriber_id);
    }

}

// impl<E> Default for EventBus<E>
//     where E: Event,
// {
//     fn default() -> Self {
//         Self::new()
//     }
// }
