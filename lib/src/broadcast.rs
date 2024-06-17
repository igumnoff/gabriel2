use std::future::Future;
use std::pin::Pin;

use crate::{
    ActorRef, SSSD,
};

use std::collections::HashMap;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use tokio::{
    sync::{
        mpsc::{
            self,
            error::{SendError, RecvError},
            Receiver, Sender
        },
        Mutex
    },
    task
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
    rx: Arc<Mutex<Receiver<E>>>,
    counter: AtomicUsize,
}

impl<E> EventBus<E>
where
    E: Event,
{
    // FIXME: Calls function only once for some reason...
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(1000);

        let event_bus = Self {
            subscribers: Arc::new(Mutex::new(HashMap::new())),
            tx: tx,
            rx: Arc::new(Mutex::new(rx)),
            counter: AtomicUsize::new(0),
        };

        log::trace!("Event bus is running");
        let subs = event_bus.subscribers.clone();
        let rcv = event_bus.rx.clone();

        let handle = tokio::runtime::Handle::current();

        {
            let _join_handler = handle.spawn(async move {
                loop {
                    EventBus::unwind_events(
                         rcv.clone(), subs.clone()
                    ).await;
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

        EventBus::unwind_events(
            self.rx.clone(),
            self.subscribers.clone()
        ).await; // This made beacause of invatiant
                 // that subscribers should see events
                 // that goes only after subscription

        let id = self.counter.fetch_add(1, Ordering::SeqCst);
        log::trace!("New subscriber in event bus. id: {}", id);

        let mut subscribers = self.subscribers.lock().await;
        subscribers.insert(id, Box::pin(callback));
        id
    }

    pub async fn unsubscribe(&self, subscriber_id: usize) {

        EventBus::unwind_events(
            self.rx.clone(),
            self.subscribers.clone()
        ).await; // This made beacause of invatiant
                 // that subscribers should see events
                 // that goes only after subscription

        log::trace!("Removed subscriber in event bus. id: {}", subscriber_id);

        let mut subscribers = self.subscribers.lock().await;
        subscribers.remove(&subscriber_id);
    }

    // Made like this because self is not Arc, but these fields are
    async fn unwind_events(
        rcv: Arc<Mutex<Receiver<E>>>,
        subscribers: Arc<Mutex<HashMap<usize, Pin<Box<dyn EventCallback<E>>>>>>
    ) {

        let mut rcv = rcv.lock().await;
        let subscribers = subscribers.lock().await;

        while !rcv.is_empty() {
            if let Some(event) = rcv.recv().await {
                log::trace!("processing event: {:?}", event);
                for (_id, callback) in &*subscribers {
                    callback.call(event).await;
                }
            }
        }

    }
}

// impl<E> Default for EventBus<E>
//     where E: Event,
// {
//     fn default() -> Self {
//         Self::new()
//     }
// }
