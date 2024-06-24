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
/// ## new
/// ```
///  // Create 5 actors in one LoadBalancer
///  let lb = LoadBalancer::new("load_balancer", 5, |id| {
///    let ar = ActorRef::new(...);
///    ar
///  });
/// ```

pub struct LoadBalancer<Actor, Message, State, Response, Error> {
    name: String,
    actor_refs: Mutex<Vec<Arc<ActorRef<Actor, Message, State, Response, Error>>>>, // For adding new actors? Do i need even need it?
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
            actor_refs: Mutex::new(instances),
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
                while let Some((msg, sender)) = rx.recv().await {
                    let actors = lb.actor_refs.lock().await;
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
}
