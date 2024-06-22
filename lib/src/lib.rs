//!
// #![doc = include_str!("../README.md")]
//!

#[cfg(feature = "remote")]
pub mod remote;

#[cfg(feature = "sink-stream")]
pub mod sink_stream;

#[cfg(feature = "broadcast")]
pub mod broadcast;

use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{mpsc, oneshot};
use futures::lock::Mutex;
use tokio::sync::oneshot::Sender;

/// `SSSD` is a trait that represents a type that is `Send`, `Sync`, `Debug`, and `'static`.
///
/// This trait is used as a bound for types that need to be sent between threads, shared references between threads,
/// have the ability to be formatted using the `Debug` formatter, and have a static lifetime.
pub trait SSSD: Send + Sync + Debug +  'static {}

/// This is an implementation of the `SSSD` trait for all types `S` that satisfy the bounds of being `Send`, `Sync`, `Debug`, and `'static`.
impl<S> SSSD for S where S: Send + Sync + Debug + 'static {}


/// `ActorRef` is a structure that represents a reference to an actor in an actor system.
/// It contains the necessary components to interact with the actor and manage its state.
///
/// # Type Parameters
///
/// * `Actor`: The type of the actor this reference points to.
/// * `Message`: The type of messages that can be sent to the actor.
/// * `State`: The type of the state that the actor maintains.
/// * `Response`: The type of the response that the actor produces.
/// * `Error`: The type of the error that the actor can return.
///
/// # Fields
///
/// * `tx`: A sender in the message-passing channel. It is used to send messages to the actor.
/// * `state`: An atomic reference counter (Arc) wrapping a mutex-protected state of the actor.
/// * `name`: A string representing the name of the actor.
/// * `actor`: An atomic reference counter (Arc) to the actor.
/// * `running`: An atomic boolean indicating whether the actor is currently running.
#[derive(Debug)]
pub struct ActorRef<Actor, Message, State, Response, Error> {
    tx: mpsc::Sender<(Message, Option<Sender<Result<Response, Error>>>)>,
    state: Arc<Mutex<State>>,
    name: String,
    actor: Arc<Actor>,
    running: AtomicBool,
}

impl<Actor, Message, State, Response, Error>  Drop for ActorRef<Actor, Message, State, Response, Error>  {
    fn drop(&mut self) {
        log::trace!("Drop actor: {}", self.name);
    }
}

/// `Context` is a structure that represents the context in which an actor operates in an actor system.
/// It contains the necessary components for an actor to process a message and manage its state.
///
/// # Type Parameters
///
/// * `Actor`: The type of the actor this context is associated with.
/// * `Message`: The type of messages that can be processed in this context.
/// * `State`: The type of the state that the actor maintains.
/// * `Response`: The type of the response that the actor produces.
/// * `Error`: The type of the error that the actor can return.
///
/// # Fields
///
/// * `mgs`: The message that the actor needs to process.
/// * `state`: An atomic reference counter (Arc) wrapping a mutex-protected state of the actor.
/// * `self_ref`: A reference to the actor itself.
#[derive(Debug)]
pub struct Context<Actor, Message, State, Response, Error> {
    pub mgs: Message,
    pub state: Arc<Mutex<State>>,
    pub self_ref: Arc<ActorRef<Actor, Message, State, Response, Error>>,
}

/// The `Handler` trait defines the behavior of an actor in an actor system.
/// It provides methods for handling incoming messages and lifecycle events.
///
/// # Type Parameters
///
/// * `Actor`: The type of the actor this handler is associated with. It must implement the `SSSD` trait.
/// * `Message`: The type of messages that this handler can process. It must implement the `SSSD` trait.
/// * `State`: The type of the state that the actor maintains. It must implement the `SSSD` trait.
/// * `Response`: The type of the response that the actor produces. It must implement the `SSSD` trait.
/// * `Error`: The type of the error that the actor can return. It must implement the `SSSD` trait and `std::error::Error`, and be convertible from `std::io::Error`.
pub trait Handler {
    type Actor: SSSD;
    type Message: SSSD;
    type State: SSSD;
    type Response: SSSD;
    type Error: SSSD + std::error::Error + From<std::io::Error>;

    fn receive(&self, ctx: Arc<Context<Self::Actor, Self::Message, Self::State, Self::Response, Self::Error>>) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send ;

    fn pre_start(&self, _state: Arc<Mutex<Self::State>>) -> impl Future<Output = Result<(), Self::Error>> {
        async {
            Ok(())
        }
    }

    fn pre_stop(&self, _state: Arc<Mutex<Self::State>>) -> impl Future<Output = Result<(), Self::Error>> {
        async {
            Ok(())
        }
    }
}

/// The `ActorTrait` trait defines the behavior of an actor in an actor system.
/// It provides methods for sending messages to the actor, asking the actor for a response, and stopping the actor.
///
/// # Type Parameters
///
/// * `Message`: The type of messages that this actor can process. It must implement the `SSSD` trait.
/// * `Response`: The type of the response that the actor produces. It must implement the `SSSD` trait.
/// * `Error`: The type of the error that the actor can return. It must implement the `SSSD` trait and `std::error::Error`, and be convertible from `std::io::Error`.
pub trait ActorTrait {
    type Message: SSSD;
    type Response: SSSD;
    type Error: SSSD + std::error::Error + From<std::io::Error>;

    fn ask(&self, msg: Self::Message) -> impl Future<Output = Result<Self::Response, Self::Error>>;
    fn send(&self, msg: Self::Message) -> impl Future<Output = Result<(), std::io::Error>>;
    fn stop(&self) ->impl Future<Output = Result<(), Self::Error>>;

}

/// The `ActorRefTrait` trait defines the behavior of an actor reference in an actor system.
/// It provides methods for creating a new actor reference and getting the state of the actor.
///
/// # Type Parameters
///
/// * `Actor`: The type of the actor this reference points to. It must implement the `Handler` and `SSSD` traits.
/// * `State`: The type of the state that the actor maintains. It must implement the `SSSD` trait.
/// * `Error`: The type of the error that the actor can return. It must implement the `SSSD` trait and `std::error::Error`, and be convertible from `std::io::Error`.
pub trait ActorRefTrait {
    type Actor:  Handler + SSSD;
    type State: SSSD;
    type Error: SSSD + std::error::Error + From<std::io::Error>;

    fn new(name: impl AsRef<str>, actor: Self::Actor, state: Self::State, buffer: usize) -> impl Future<Output = Result<Arc<Self>, Self::Error>>;
    fn state(&self) -> impl Future<Output = Result<Arc<Mutex<Self::State>>, std::io::Error>>;
}


/// Implementation of the `ActorTrait` for `ActorRef`.
///
/// This implementation provides the functionality for sending messages to the actor (`ask` and `send` methods),
/// and for stopping the actor (`stop` method).
///
/// # Type Parameters
///
/// * `Actor`: The type of the actor this reference points to. It must implement the `Handler` and `SSSD` traits.
/// * `Message`: The type of messages that this actor can process. It must implement the `SSSD` trait.
/// * `State`: The type of the state that the actor maintains. It must implement the `SSSD` trait.
/// * `Response`: The type of the response that the actor produces. It must implement the `SSSD` trait.
/// * `Error`: The type of the error that the actor can return. It must implement the `SSSD` trait and `std::error::Error`, and be convertible from `std::io::Error`.
impl <Actor: Handler<Actor = Actor, State = State, Message = Message, Error = Error, Response = Response> + SSSD,
    Message: SSSD, State: SSSD, Response:  SSSD, Error: SSSD + std::error::Error + From<std::io::Error>> ActorTrait for ActorRef<Actor, Message, State, Response, Error> {
    type Message = Message;
    type Response = Response;
    type Error = Error;
    async fn ask(&self, msg: Message) -> Result<Response, Error>
    {
        log::debug!("<{}> Result message: {:?}", self.name, msg);

        let (sender, receiver) = oneshot::channel();
        {
            let r = self.tx.send((msg, Some(sender))).await;
            if r.is_err() {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "Err").into());
            }
        }
        let r = receiver.await;
        match r {
            Ok(res) => { res }
            Err(_) => {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "Err").into());
            }
        }
    }

    async fn send(&self, msg: Message) -> Result<(), std::io::Error> {
        log::debug!("<{}> Push message: {:?}", self.name, msg);
        let r = self.tx.send((msg, None)).await;
        if r.is_err() {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Err").into());
        }
        Ok(())
    }

    async fn stop(&self) -> Result<(), Error> {
        if self.running.load(Ordering::SeqCst) == false {
            return Ok(());
        }
        self.actor.pre_stop(self.state.clone()).await?;

        self.running.store(false, Ordering::SeqCst);
        log::debug!("<{}> Stop worker", self.name);
        Ok(())
    }
}

/// Implementation of the `ActorRefTrait` for `ActorRef`.
///
/// This implementation provides the functionality for creating a new actor reference (`new` method),
/// and for getting the state of the actor (`state` method).
///
/// # Type Parameters
///
/// * `Actor`: The type of the actor this reference points to. It must implement the `Handler` and `SSSD` traits.
/// * `Message`: The type of messages that this actor can process. It must implement the `SSSD` trait.
/// * `State`: The type of the state that the actor maintains. It must implement the `SSSD` trait.
/// * `Response`: The type of the response that the actor produces. It must implement the `SSSD` trait.
/// * `Error`: The type of the error that the actor can return. It must implement the `SSSD` trait and `std::error::Error`, and be convertible from `std::io::Error`.
impl <Actor: Handler<Actor = Actor, State = State, Message = Message, Error = Error, Response = Response> + SSSD , Message: SSSD,
    State: SSSD, Response:  SSSD, Error: SSSD + std::error::Error + From<std::io::Error>> ActorRefTrait for ActorRef<Actor, Message, State, Response, Error> {
    type Actor = Actor;
    type State = State;
    type Error = Error;

    async fn new(name: impl AsRef<str>, actor: Self::Actor, state: Self::State, buffer: usize) ->  Result<Arc<Self>, Self::Error>
    {
        let state_arc = Arc::new(Mutex::new(state));
        let state_clone = state_arc.clone();
        let (tx, mut rx) = mpsc::channel(buffer);
        let actor_arc= Arc::new(actor);
        let actor = actor_arc.clone();
        let actor_ref = ActorRef {
            tx,
            state: state_clone,
            name: name.as_ref().to_string(),
            actor:actor_arc.clone(),
            running: AtomicBool::new(false),
        };

        let ret = Arc::new(actor_ref);
        let ret_clone = ret.clone();
        let ret_clone2 = ret.clone();
        let ret_clone3 = ret.clone();

        let handle = tokio::runtime::Handle::current();
        let _ = actor_arc.pre_start(ret_clone.state.clone()).await?;
        ret.running.store(true, Ordering::SeqCst);
        let _ = handle.spawn(async move {
            let me = ret_clone2.clone();
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                        if ret_clone3.running.load(Ordering::SeqCst) == false {
                            break;
                        }
                    },
                    msg_opt = rx.recv() => {
                        match msg_opt {
                            None => {
                                log::debug!("<{}> No message", me.name);
                                break;
                            }
                            Some(message) => {
                                if ret_clone3.running.load(Ordering::SeqCst ) == false {
                                    break;
                                }
                                let msg = message.0;
                                let sender = message.1;
                                let state_clone = state_arc.clone();
                                log::debug!("<{}> Got message: {:?} Current state: {:?}", me.name, msg, state_clone.lock().await);
                                let msg_debug = format!("{:?}", msg);
                                let state = state_arc.clone();
                                let context = Context {
                                    mgs: msg,
                                    state: state,
                                    self_ref: me.clone(),
                                };

                                let r = actor.receive(Arc::new(context));
                                {
                                    let result = r.await;
                                    log::trace!("<{}> Work result: {:?}", me.name, result);
                                    if result.is_err() {
                                        log::error!("<{}> Work error: {:?} on message {}", me.name, result, msg_debug);
                                    }
                                    if let Some(sender) = sender {
                                            log::trace!("<{}> Promise result: {:?}", me.name, result);
                                            let _ = sender.send(result);
                                    } else {
                                        log::trace!("<{}> No promise", me.name);
                                    }
                                }
                                let state_clone = state_arc.clone();
                                log::trace!("<{}> After work on message new state: {:?}", me.name, state_clone.lock().await);
                            }
                        };
                    }
                }
            }
            log::debug!("Actor <{}> stopped", me.name);
        });
        log::info!("<{}> Actor started", ret_clone.name);
        Ok(ret_clone)
    }


    async fn state(&self) -> Result<Arc<Mutex<Self::State>>, std::io::Error> {
        let state = self.state.clone();
        log::trace!("<{}> State: {:?}", self.name, state.lock().await);
        Ok(state)
    }

}
