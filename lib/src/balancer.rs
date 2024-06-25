use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use std::sync::Arc;

use tokio::sync::{mpsc, oneshot, Mutex};

use crate::{ActorRef, ActorRefTrait, ActorTrait, Handler, SSSD};

/// `LoadBalancer` is a proxy that distributes load among a collection of `ActorRef` instances.
///
/// # Methods
///
/// * `new` - Creates a `LoadBalancer` with the specified number of instances, which are built using the given closure.
/// * `send` - Sends a message without waiting for a response.
/// * `ask` - Sends a message and waits for a response.
/// * `stop` - Deactivates the `LoadBalancer` and all associated actors.
/// * `state` - Retrieves the state of a specified actor identified by its ID.
///
/// # Examples
///
pub struct LoadBalancer<Actor, Message, State, Response, Error> {
    name: String,
    actor_refs: Arc<Vec<Arc<ActorRef<Actor, Message, State, Response, Error>>>>,
    tx: mpsc::Sender<(Message, Option<oneshot::Sender<Result<Response, Error>>>)>,
    running: AtomicBool,
    turn: AtomicUsize,
}

impl<
        Actor: Handler<
                Actor = Actor,
                State = State,
                Response = Response,
                Message = Message,
                Error = Error,
            > + SSSD,
        Message: SSSD,
        State: SSSD,
        Response: SSSD,
        Error: SSSD + std::error::Error + From<std::io::Error>,
    > LoadBalancer<Actor, Message, State, Response, Error>
{
    pub async fn new<F>(
        name: impl Into<String>,
        instances_amount: usize,
        builder: F,
    ) -> Result<Arc<Self>, Error>
    where
        F: Fn(usize) -> Result<Arc<ActorRef<Actor, Message, State, Response, Error>>, Error>,
    {
        let mut instances = vec![];

        for id in 0..instances_amount {
            instances.push(builder(id)?);
        }

        let (tx, mut rx) = mpsc::channel(1000);

        let lb = Arc::new(Self {
            name: name.into(),
            actor_refs: Arc::new(instances),
            tx: tx,
            running: AtomicBool::new(false),
            turn: AtomicUsize::new(0),
        });

        let lb_clone = lb.clone();
        let handler = tokio::runtime::Handle::current();
        handler.spawn(async move {
            let lb = lb_clone.clone();
            lb.running.store(true, Ordering::SeqCst);
            // TODO: Shutdown on .stop()
            loop {
                if !lb.running.load(Ordering::SeqCst) {
                    break;
                }
                while let Some((msg, sender)) = rx.recv().await {
                    let actors = lb.actor_refs.clone();
                    let turn = lb
                        .turn
                        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| {
                            Some((x + 1) % actors.len())
                        });

                    let turn = match turn {
                        Err(val) => {
                            log::error!("Failed to update LoadBalancer.turn");
                            val
                        }
                        Ok(val) => val,
                    };

                    let msg_dbg = format!("{:?}", msg);
                    if let Some(sender) = sender {
                        // Case: Asked
                        let msg_back = actors[turn].ask(msg).await;
                        let result = sender.send(msg_back);
                        if result.is_err() {
                            log::error!(
                                "<{}> Error: {:?} on message: {}",
                                lb.name,
                                result,
                                msg_dbg
                            );
                        }
                    } else {
                        // Case: Send
                        let result = actors[turn].send(msg).await;
                        if result.is_err() {
                            log::error!(
                                "<{}> Error: {:?} on message: {}",
                                lb.name,
                                result,
                                msg_dbg
                            );
                        }
                    }
                }
            }
        });

        Ok(lb)
    }

    pub async fn send(&self, msg: Message) -> Result<(), std::io::Error> {
        log::debug!("<{}> Push message: {:?}", self.name, msg);
        let r = self.tx.send((msg, None)).await;
        if r.is_err() {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Err").into());
        }
        Ok(())
    }

    pub async fn ask(&self, msg: Message) -> Result<Response, Error> {
        log::debug!("<{}> Push message: {:?}", self.name, msg);
        let (sender, reciever) = oneshot::channel();
        {
            let r = self.tx.send((msg, Some(sender))).await;
            if r.is_err() {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "Err").into());
            }
        }
        let response = reciever.await;

        match response {
            Ok(r) => r,
            Err(_) => {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "Err").into());
            }
        }
    }

    pub async fn stop(&self) -> Result<(), Error> {
        if self.running.load(Ordering::SeqCst) == false {
            return Ok(());
        }
        for actor_ref in &*self.actor_refs {
            actor_ref.stop().await?;
        }
        Ok(())
    }

    pub async fn state(
        &self,
        id: usize,
    ) -> Result<Arc<futures::lock::Mutex<State>>, std::io::Error> {
        // Is there a difference between futures::lock::Mutex and tokio::sync::Mutex?
        self.actor_refs[id].state().await
    }
}

#[cfg(test)]
mod balancer_test {
    use super::*;
    #[tokio::test]
    async fn create() {
        assert_eq!(1, 1);
    }
}
