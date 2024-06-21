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
use tokio::sync::{mpsc, oneshot};
use futures::lock::Mutex;
use tokio::sync::oneshot::Sender;

pub trait SSSD: Send + Sync + Debug +  'static {}
impl<S> SSSD for S where S: Send + Sync + Debug + 'static {}


#[derive(Debug)]
pub struct ActorRef<Actor, Message, State, Response, Error> {
    tx: mpsc::Sender<(Message, Option<Sender<Result<Response, Error>>>)>,
    state: Arc<Mutex<State>>,
    name: String,
    actor: Arc<Actor>,
    running: Mutex<bool>,
}

impl<Actor, Message, State, Response, Error>  Drop for ActorRef<Actor, Message, State, Response, Error>  {
    fn drop(&mut self) {
        log::trace!("Drop actor: {}", self.name);
    }
}

#[derive(Debug)]
pub struct Context<Actor, Message, State, Response, Error> {
    pub mgs: Message,
    pub state: Arc<Mutex<State>>,
    pub self_ref: Arc<ActorRef<Actor, Message, State, Response, Error>>,
}

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
        if *self.running.lock().await == false {
            return Ok(());
        }
        self.actor.pre_stop(self.state.clone()).await?;

        *self.running.lock().await = false;
        log::debug!("<{}> Stop worker", self.name);
        Ok(())
    }
}

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
            running: Mutex::new(false),
        };

        let ret = Arc::new(actor_ref);
        let ret_clone = ret.clone();
        let ret_clone2 = ret.clone();
        let ret_clone3 = ret.clone();

        let handle = tokio::runtime::Handle::current();
        let _ = handle.spawn(async move {
            let me = ret_clone2.clone();
            loop {
                tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                    if *ret_clone3.running.lock().await == false {
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
                                if *ret_clone3.running.lock().await == false {
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
        let _ = actor_arc.pre_start(ret_clone.state.clone()).await?;
        *ret.running.lock().await = true;
        log::info!("<{}> Actor started", ret_clone.name);
        Ok(ret_clone)
    }


    async fn state(&self) -> Result<Arc<Mutex<Self::State>>, std::io::Error> {
        let state = self.state.clone();
        log::trace!("<{}> State: {:?}", self.name, state.lock().await);
        Ok(state)
    }

}
