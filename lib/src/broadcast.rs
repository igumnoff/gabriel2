use std::future::Future;
use async_trait::async_trait;
use std::pin::Pin;
use crate::SSSD;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use tokio::sync::Mutex;
use std::ops::Drop;
use tokio::task::{JoinHandle, self};
use std::collections::HashMap;

use tokio::sync::watch::{
    self,
    Sender,
    Receiver,
    error::{ SendError, RecvError }
};


/// Marker-trait for events
pub trait Event: Copy + Clone + SSSD {}

/// `EventCallback` is a marker-trait for callback which will be stored in subscribers
/// Auto-implemented
#[async_trait]
pub trait EventCallback<E>: Send + Sync + 'static {
    async fn call(&self, event: E);
}

#[async_trait]
impl<F, E, Fut> EventCallback<E> for F
    where
        F: Fn(E) -> Fut + Send + Sync + 'static,
        E: Event,
        Fut: Future<Output = ()> + Send + 'static
{
    async fn call(&self, event: E) {
        self(event).await;
    }
}

/// # Event Bus
///
/// `EventBus<E,R>`, where `E` implements `Event` marker trait `R` `implements EventCallback<E>`
///
/// `EventBus` accessible from all actors
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
/// * `publish` - Sends an event to a `events`
///
/// * `subscribe` - Creates record in `subscribers` of enum variant and callback to that variant and returns subscriber's id
///
/// * `unsubscribe` - Remove record from `subscribers`
///
/// * `run` - run through each event and execute callbacks
///
pub struct EventBus<E>
where E: Event
{
    subscribers: Arc<Mutex<HashMap<usize, Pin<Box<dyn EventCallback<E>>>>>>,
    tx:          Sender<Option<E>>,
    rx:          Arc<Mutex<Receiver<Option<E>>>>,
    counter:     AtomicUsize
}

impl<E> EventBus<E>
    where E: Event,
{

    // FIXME: Calls function only once for some reason...
    pub fn new() -> Self {

        let (tx, rx) = watch::channel::<Option<E>>(None);

        let event_bus = Self {
            subscribers: Arc::new(Mutex::new(HashMap::new())),
            tx: tx,
            rx: Arc::new(Mutex::new(rx)),
            counter: AtomicUsize::new(0)
        };

        log::trace!("Event bus is running");
        let subs = event_bus.subscribers.clone();
        let rcv  = event_bus.rx.clone();

        let handle = tokio::runtime::Handle::current();

        {
            let _join_handler = handle.spawn(async move {
                loop {
                    let subscribers = subs.lock().await;
                    let mut rcv = rcv.lock().await;

                    if rcv.changed().await.is_ok()
                    {
                        let event = *rcv.borrow_and_update();
                        for (id, callback) in &*subscribers {
                            if let Some(e) = event {
                                log::trace!("Performing callback for subscriber id: {}, event: {:?}", id, e);
                                callback.call(e).await;
                            }
                        }
                    } else {
                        println!("W");
                    }
                    // Reset rcv
                }
            });
        } // Dropped to make it background task

        event_bus

    }

    pub async fn publish(&self, event: E) -> Result<(), SendError<Option<E>>> {
        self.tx.send(Some(event))?;
        log::trace!("New event in event bus: {:?}", event);
        Ok(())
    }

    pub async fn subscribe<F>(&self, callback: F) -> usize
        where F: EventCallback<E>
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
