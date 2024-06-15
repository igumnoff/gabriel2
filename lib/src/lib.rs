//!
// #![doc = include_str!("../README.md")]
//!

#[cfg(feature = "remote")]
pub mod remote;

#[cfg(feature = "sink-stream")]
pub mod sink_stream;
#[cfg(feature = "broadcast")]
pub mod broadcast;

use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use futures::lock::Mutex;
use tokio::sync::oneshot::Sender;

pub trait SSSD: Send + Sync + Debug +  'static {}
impl<S> SSSD for S where S: Send + Sync + Debug + 'static {}


/// The `ActorRef` struct represents a reference to an actor in the Actorlib framework.
/// It contains the following fields:
///
/// - `tx`: A `Mutex` wrapping an `Option` that contains a sender part of a message passing channel.
///   This is used to send messages to the actor's task.
///
/// - `join_handle`: A `Mutex` wrapping an `Option` that contains a handle to the actor's task.
///   This is used to control the execution of the actor's task.
///
/// - `state`: An `Option` that contains the current state of the actor, wrapped in an `Arc<Mutex>` for thread safety.
///
/// - `self_ref`: A `Mutex` wrapping an `Option` that contains a reference to the actor itself.
///   This is useful for actors that need to send messages to themselves.
///
/// - `message_id`: A `Mutex` wrapping an integer that represents the ID of the last message sent to the actor.
///
/// - `promise`: A `Mutex` wrapping a `HashMap` that maps message IDs to one-shot channel senders.
///   This is used to send responses back to the sender of a message.
///
/// - `name`: A `String` that represents the name of the actor.
///
/// - `actor`: An `Arc` that contains the actor.
///
/// - `running`: A `Mutex` wrapping a boolean that indicates whether the actor is currently running or not.
///
/// # Type parameters
/// - `Actor`: The type of the actor.
/// - `Message`: The type of the messages that the actor can process.
/// - `State`: The type of the state of the actor.
/// - `Response`: The type of the response that the actor produces after processing a message.
/// - `Error`: The type of the error that the actor can produce.
#[derive(Debug)]
pub struct ActorRef<Actor, Message, State, Response, Error> {
    tx: Mutex<Option<mpsc::Sender<(Message, u64)>>>,
    join_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    state: Option<Arc<Mutex<State>>>,
    self_ref: Mutex<Option<Arc<ActorRef<Actor, Message, State, Response, Error>>>>,
    message_id: Mutex<u64>,
    promise: Mutex<HashMap<u64, Sender<Result<Response, Error>>>>,
    name: String,
    actor: Mutex<Option<Arc<Actor>>>,
    running: Mutex<bool>,
}

impl<Actor, Message, State, Response, Error>  Drop for ActorRef<Actor, Message, State, Response, Error>  {
    fn drop(&mut self) {
        log::trace!("Drop actor: {}", self.name);
    }
}

/// `Context` is a structure that holds the context of an actor.
///
/// It contains the following fields:
/// - `mgs`: The message that the actor is currently processing.
/// - `state`: The current state of the actor, wrapped in an `Arc<Mutex<T>>` for thread safety.
/// - `self_ref`: A reference to the actor itself. This is useful for actors that need to send messages to themselves.
///
/// # Type parameters
/// - `Actor`: The type of the actor.
/// - `Message`: The type of the messages that the actor can process.
/// - `State`: The type of the state of the actor.
/// - `Response`: The type of the response that the actor produces after processing a message.
/// - `Error`: The type of the error that the actor can produce.
#[derive(Debug)]
pub struct Context<Actor, Message, State, Response, Error> {
    pub mgs: Message,
    pub state: Arc<Mutex<State>>,
    pub self_ref: Arc<ActorRef<Actor, Message, State, Response, Error>>,
}

/// `Handler` is a trait that defines the behavior of an actor in the Actorlib framework.
///
/// This trait is implemented by any type that can act as an actor in the Actorlib framework.
/// It provides methods for handling messages and managing the actor's lifecycle.
///
/// # Type parameters
/// - `Actor`: The type of the actor. This type must be `Sync`, `Send` and `'static`.
/// - `Message`: The type of the messages that the actor can process. This type must be `Sync`, `Send` and `'static`.
/// - `State`: The type of the state of the actor. This type must be `Sync`, `Send` and `'static`.
/// - `Response`: The type of the response that the actor produces after processing a message. This type must be `Sync`, `Send` and `'static`.
/// - `Error`: The type of the error that the actor can produce. This type must be `Sync`, `Send` and `'static`.
///
/// # Methods
/// - `receive`: This method is called when the actor receives a message. It takes a context, which contains the message and the state of the actor, and returns a `Future` that resolves to a `Result` containing either the response produced by the actor after processing the message, or an error.
/// - `pre_start`: This method is called before the actor starts. It takes the state of the actor and a reference to the actor itself, and returns a `Future` that resolves to a `Result`. If the `Result` is `Ok`, the actor starts; if it is `Err`, the actor does not start. By default, this method returns `Ok(())`.
/// - `pre_stop`: This method is called before the actor stops. It takes the state of the actor and a reference to the actor itself, and returns a `Future` that resolves to a `Result`. If the `Result` is `Ok`, the actor stops; if it is `Err`, the actor does not stop. By default, this method returns `Ok(())`.
pub trait Handler {
    type Actor: SSSD;
    type Message: SSSD;
    type State: SSSD;
    type Response: SSSD;
    type Error: SSSD + std::error::Error + From<std::io::Error>;

    fn receive(&self, ctx: Arc<Context<Self::Actor, Self::Message, Self::State, Self::Response, Self::Error>>) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send ;
    fn pre_start(&self, _state: Arc<Mutex<Self::State>>, _self_ref: Arc<ActorRef<Self::Actor, Self::Message, Self::State, Self::Response, Self::Error>>) -> impl Future<Output = Result<(), Self::Error>> {
        async {
            Ok(())
        }
    }
    fn pre_stop(&self, _state: Arc<Mutex<Self::State>>, _self_ref: Arc<ActorRef<Self::Actor, Self::Message, Self::State, Self::Response, Self::Error>>) -> impl Future<Output = Result<(), Self::Error>> {
        async {
            Ok(())
        }
    }
}

pub trait ActorTrait {
    type Message: SSSD;
    type Response: SSSD;
    type Error: SSSD + std::error::Error + From<std::io::Error>;

    fn ask(&self, msg: Self::Message) -> impl Future<Output = Result<Self::Response, Self::Error>>;
    fn send(&self, msg: Self::Message) -> impl Future<Output = Result<(), std::io::Error>>;
    fn stop(&self) ->impl Future<Output = Result<(), Self::Error>>;

}


pub trait ActorRefTrait {
    type Actor:  Handler + SSSD;
    type State: SSSD;
    type Error: SSSD + std::error::Error + From<std::io::Error>;

    fn new(name: impl AsRef<str>, actor: Self::Actor, state: Self::State, buffer: usize) -> impl Future<Output = Result<Arc<Self>, Self::Error>>;
    fn state(&self) -> impl Future<Output = Result<Arc<Mutex<Self::State>>, std::io::Error>>;
}


impl <Actor: Handler<Actor = Actor, State = State, Message = Message, Error = Error, Response = Response> + SSSD,
    Message: SSSD, State: SSSD, Response:  SSSD, Error: SSSD + std::error::Error + From<std::io::Error>> ActorTrait for ActorRef<Actor, Message, State, Response, Error> {
    type Message = Message;
    type Response = Response;
    type Error = Error;


    /// Sends a message to the actor and waits for a response.
    ///
    /// This method is part of the `ActorRef` struct and is used to send a message to the actor
    /// and wait for a response. This is done by sending the message and a unique message ID
    /// through a channel to the actor's task, which processes the message and sends the response
    /// back through a one-shot channel.
    ///
    /// The method locks the `tx` field of the `ActorRef` struct, which is an `Option` containing
    /// a sender part of a message passing channel. If the `tx` field is `None`, the method returns
    /// an error. Otherwise, it creates a new one-shot channel for the response, increments the
    /// message ID, inserts the sender part of the one-shot channel into the `promise` field of the
    /// `ActorRef` struct (which is a `HashMap` mapping message IDs to one-shot channel senders),
    /// and sends the message and the message ID through the `tx` channel.
    ///
    /// After sending the message, the method awaits the receiver part of the one-shot channel.
    /// If the receiver receives a response, the method returns the response. If the receiver
    /// is closed without sending a response, the method returns an error.
    ///
    /// # Parameters
    /// - `msg`: The message to send to the actor.
    ///
    /// # Returns
    /// - `Result<Response, Error>`: The response from the actor, or an error if the actor failed
    ///   to process the message or if the `tx` field is `None`.
    async fn ask(&self, msg: Message) -> Result<Response, Error>
    {
        log::debug!("<{}> Result message: {:?}", self.name, msg);

        let counter = {
            let mut counter = self.message_id.lock().await;
            *counter += 1;
            *counter
        };
        let tx_lock = self.tx.lock().await;
        let tx = tx_lock.as_ref();
        match tx {
            None => {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "Err").into());
            }
            Some(tx) => {
                let (sender, receiver) = oneshot::channel();
                {
                    self.promise.lock().await.insert(counter, sender);
                    let r = tx.send((msg, counter)).await;
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
        }
    }

    /// Sends a message to the actor without waiting for a response.
    ///
    /// This method is part of the `ActorRef` struct and is used to send a message to the actor
    /// without waiting for a response. This is done by sending the message and a zero message ID
    /// through a channel to the actor's task, which processes the message.
    ///
    /// The method locks the `tx` field of the `ActorRef` struct, which is an `Option` containing
    /// a sender part of a message passing channel. If the `tx` field is `None`, the method returns
    /// an error. Otherwise, it sends the message and a zero message ID through the `tx` channel.
    ///
    /// After sending the message, the method returns `Ok(())` if the message was sent successfully,
    /// or an error if the message could not be sent.
    ///
    /// # Parameters
    /// - `msg`: The message to send to the actor.
    ///
    /// # Returns
    /// - `Result<(), std::io::Error>`: `Ok(())` if the message was sent successfully, or an error
    ///   if the message could not be sent or if the `tx` field is `None`.
    async fn send(&self, msg: Message) -> Result<(), std::io::Error> {
        log::debug!("<{}> Push message: {:?}", self.name, msg);
        let tx_lock = self.tx.lock().await;
        let tx = tx_lock.as_ref();
        match tx {
            None => {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "Err").into());
            }
            Some(tx) => {
                let r = tx.send((msg, 0)).await;
                if r.is_err() {
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, "Err").into());
                }
                // let r = tx.try_send((msg, 0));
                // match r {
                //     Ok(_) => {}
                //     Err(err) => {
                //         let err_str = format!("{:?}", err);
                //         return Err(std::io::Error::new(std::io::ErrorKind::Other, err_str).into());
                //     }
                // }
            }
        }
        Ok(())
    }

    /// Stops the actor.
    ///
    /// This method is part of the `ActorRef` struct and is used to stop the actor's execution.
    ///
    /// The method performs the following steps:
    /// 1. Checks if the actor is already stopped. If it is, the method returns `Ok(())`.
    /// 2. Calls the `pre_stop` method of the actor.
    /// 3. Sets the `running` flag of the `ActorRef` to `false`.
    /// 4. Clears the `tx` and `self_ref` fields of the `ActorRef`.
    /// 5. Aborts the actor's task if it is running.
    /// 6. Clears the `join_handle` field of the `ActorRef`.
    /// 7. Logs a debug message indicating that the actor has stopped.
    ///
    /// # Returns
    /// - `Result<(), Error>`: `Ok(())` if the actor was stopped successfully, or an `Error` if the `pre_stop` method of the actor returned an error.
    async fn stop(&self) -> Result<(), Error> {
        if *self.running.lock().await == false {
            return Ok(());
        }
        {
            let self_ref =  self.self_ref.lock().await.clone().unwrap();
            let mut lock = self.actor.lock().await;
            let actor = lock.as_ref().unwrap();
            actor.pre_stop(self.state.clone().unwrap(), self_ref).await?;
            *lock = None;
        }
        *self.self_ref.lock().await = None;

        *self.running.lock().await = false;
        *self.tx.lock().await = None;
        *self.self_ref.lock().await = None;
        let join_handle = self.join_handle.lock().await.take();
        match join_handle {
            None => {}
            Some(join_handle) => {
                let _ = join_handle.abort();
                log::trace!("join_handle abort()");
            }
        }
        *self.join_handle.lock().await = None;
        *self.promise.lock().await = HashMap::new();
        log::debug!("<{}> Stop worker", self.name);
        Ok(())
    }
}

impl <Actor: Handler<Actor = Actor, State = State, Message = Message, Error = Error, Response = Response> + SSSD , Message: SSSD,
    State: SSSD, Response:  SSSD, Error: SSSD + std::error::Error + From<std::io::Error>> ActorRefTrait for ActorRef<Actor, Message, State, Response, Error> {
    type Actor = Actor;
    type State = State;
    type Error = Error;

    /// Creates a new `ActorRef` instance.
    ///
    /// This function takes the following parameters:
    /// - `name`: A `String` that represents the name of the actor.
    /// - `actor`: An instance of the actor.
    /// - `state`: The initial state of the actor.
    /// - `buffer`: The size of the message buffer.
    ///
    /// The function returns a `Result` that contains an `Arc` to the `ActorRef` instance in case of success,
    /// or an `Error` in case of failure.
    ///
    /// The function performs the following steps:
    /// 1. Wraps the initial state in an `Arc<Mutex>`.
    /// 2. Creates a new message passing channel with the specified buffer size.
    /// 3. Creates a new `ActorRef` instance and wraps it in an `Arc`.
    /// 4. Spawns a new asynchronous task that listens for incoming messages and processes them.
    /// 5. Calls the `pre_start` method of the actor.
    /// 6. Sets the `running` flag of the `ActorRef` to `true`.
    /// 7. Returns the `Arc` to the `ActorRef` instance.
    ///
    /// # Type parameters
    /// - `Actor`: The type of the actor. This type must implement the `Handler` trait and be `Sync`, `Send`, and `'static`.
    /// - `Message`: The type of the messages that the actor can process. This type must be `Debug`, `Send`, and `Sync`.
    /// - `State`: The type of the state of the actor. This type must be `Debug`, `Send`, and `Sync`.
    /// - `Response`: The type of the response that the actor produces after processing a message. This type must be `Debug`, `Send`, and `Sync`.
    /// - `Error`: The type of the error that the actor can produce. This type must implement the `std::error::Error` trait and be `Debug`, `Send`, `Sync`, and `From<std::io::Error>`.
    async fn new(name: impl AsRef<str>, actor: Self::Actor, state: Self::State, buffer: usize) ->  Result<Arc<Self>, Self::Error>
    {
        let state_arc = Arc::new(Mutex::new(state));
        let state_clone = state_arc.clone();
        let (tx, mut rx) = mpsc::channel(buffer);
        let actor_arc= Arc::new(actor);
        let actor = actor_arc.clone();
        let actor_ref = ActorRef {
            tx: Mutex::new(Some(tx)),
            join_handle: Mutex::new(None),
            state: Some(state_clone),
            self_ref: Mutex::new(None),
            message_id: Mutex::new(0),
            promise: Mutex::new(HashMap::new()),
            name: name.as_ref().to_string(),
            actor: Mutex::new(Some(actor_arc.clone())),
            running: Mutex::new(false),
        };

        let ret = Arc::new(actor_ref);
        let ret_clone = ret.clone();
        let ret_clone2 = ret.clone();
        let ret_clone3 = ret.clone();
        *ret.self_ref.lock().await = Some(ret.clone());

        let handle = tokio::runtime::Handle::current();
        let join_handle = handle.spawn(async move {
            let me = ret_clone2.clone();

            loop {
                tokio::select! {
            _ = futures::future::pending::<()>() => {

            },
            msg_opt = rx.recv() => {
                match msg_opt {
                    None => {
                        log::debug!("<{}> No message", me.name);
                        break;
                    }
                    Some(message) => {
                                if *ret_clone3.running.lock().await == false {
                                    return ();
                                }
                                let msg = message.0;
                                let message_id = message.1;
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
                                    let promise = ret_clone3.promise.lock().await.remove(&message_id);
                                    match promise {
                                        None => {
                                            log::trace!("<{}> No promise for message_id: {}", me.name, message_id);
                                        }
                                        Some(promise) => {
                                            log::trace!("<{}> Promise result: {:?}", me.name, result);
                                            let _ = promise.send(result);
                                        }
                                    }
                                }
                                let state_clone = state_arc.clone();
                                log::trace!("<{}> After work on message new state: {:?}", me.name, state_clone.lock().await);
                            }
                        };
                    }
                }
            }
        });
        *ret.join_handle.lock().await = Some(join_handle);
        let _ = actor_arc.pre_start(ret_clone.state.clone().unwrap(), ret_clone.clone()).await?;
        *ret.running.lock().await = true;
        log::info!("<{}> Actor started", ret_clone.name);
        Ok(ret_clone)
    }


    /// Retrieves the current state of the actor.
    ///
    /// This method is part of the `ActorRef` struct and is used to retrieve the current state of the actor.
    /// The state is stored in the `state` field of the `ActorRef` struct, which is an `Option` containing
    /// the state wrapped in an `Arc<Mutex>`.
    ///
    /// The method clones the `state` field and logs a trace message containing the name of the actor and the state.
    /// If the `state` field is `None`, the method returns an error. Otherwise, it returns the state.
    ///
    /// # Returns
    /// - `Result<Arc<Mutex<State>>, std::io::Error>`: The current state of the actor wrapped in an `Arc<Mutex>`,
    ///   or an error if the `state` field is `None`.
    async fn state(&self) -> Result<Arc<Mutex<Self::State>>, std::io::Error> {
        let state_opt = self.state.clone();
        log::trace!("<{}> State: {:?}", self.name, state_opt);
        match state_opt {
            None => {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "No state").into());
            }
            Some(state) => {
                Ok(state)
            }
        }
    }

}


