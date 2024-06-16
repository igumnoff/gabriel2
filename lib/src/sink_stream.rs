use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use async_stream::__private::AsyncStream;
use futures::sink::Sink;
use tokio::sync::mpsc::Sender;
use crate::{ActorTrait, Handler};
use crate::SSSD;
use crate::ActorRef;
pub trait ActorSinkTrait {
    type Actor: SSSD + Handler;
    type Message: SSSD;
    type State: SSSD;
    type Response: SSSD;
    type Error: SSSD + std::error::Error + From<std::io::Error>;
    fn sink(actor_ref: Arc<ActorRef<Self::Actor, Self::Message, Self::State, Self::Response, Self::Error>>)
            -> impl Sink<Self::Message, Error=Self::Error>
        where ActorRef<Self::Actor, Self::Message, Self::State, Self::Response, Self::Error>: ActorTrait;
}


impl <Actor: Handler<Actor = Actor, Message = Message, State = State,Response = Response, Error = Error> + SSSD,
    Message: SSSD, State: SSSD, Response: SSSD, Error:SSSD + std::error::Error + From<std::io::Error>> ActorSinkTrait for ActorSink<Actor, Message, State, Response, Error> {
    type Actor = Actor;
    type Message = Message;
    type State = State;
    type Response = Response;
    type Error = Error;
    fn sink(actor_ref: Arc<ActorRef<Self::Actor, Self::Message, Self::State, Self::Response, Self::Error>>) -> impl Sink<Self::Message, Error=Self::Error>
        where ActorRef<Self::Actor, Self::Message, Self::State, Self::Response, Self::Error>: ActorTrait
    {
        ActorSink {
            actor_ref
        }
    }
}

pub struct ActorSink<Actor, Message, State, Response, Error>
    where ActorRef<Actor, Message, State, Response, Error>: ActorTrait{
    actor_ref: Arc<ActorRef<Actor, Message, State, Response, Error>>
}


impl <Actor:Handler<Actor = Actor, State = State, Message = Message, Response = Response, Error = Error> + SSSD, Message: SSSD,
    State: SSSD, Response: SSSD, Error: SSSD + std::error::Error + From<std::io::Error>> Sink<Message> for ActorSink<Actor, Message, State, Response, Error> {
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Message) -> Result<(), Error> {
        let handle = tokio::runtime::Handle::current();
        let reference = self.actor_ref.clone();
        handle.spawn(async move {
            let _ = reference.send(item).await;
        });
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}


pub trait ActorSinkStreamTrait {
    type Actor: SSSD + Handler;
    type Message: SSSD;
    type State: SSSD;
    type Response: SSSD;
    type Error: SSSD + std::error::Error + From<std::io::Error>;
    fn sink_stream(actor_ref: Arc<ActorRef<Self::Actor, Self::Message, Self::State, Self::Response, Self::Error>>)
            -> (impl Sink<Self::Message, Error=Self::Error>, AsyncStream<Self::Response, impl Future<Output =()> + Sized>)
        where ActorRef<Self::Actor, Self::Message, Self::State, Self::Response, Self::Error>: ActorTrait;
}


pub struct ActorSinkAsk<Actor, Message, State, Response, Error>
    where ActorRef<Actor, Message, State, Response, Error>: ActorTrait{
    actor_ref: Arc<ActorRef<Actor, Message, State, Response, Error>>,
    tx: Sender<Response>
}


impl <Actor:Handler<Actor = Actor, State = State, Message = Message, Response = Response, Error = Error> + SSSD, Message: SSSD,
    State: SSSD, Response: SSSD, Error: SSSD + std::error::Error + From<std::io::Error>> Sink<Message> for ActorSinkAsk<Actor, Message, State, Response, Error> {
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Message) -> Result<(), Error> {
        let handle = tokio::runtime::Handle::current();
        let reference = self.actor_ref.clone();
        let tx = self.tx.clone();
        handle.spawn(async move {
            let response = reference.ask(item).await;
            tx.send(response.unwrap()).await.unwrap();
        });
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}

impl <Actor: Handler<Actor = Actor, Message = Message, State = State,Response = Response, Error = Error> + SSSD,
    Message: SSSD, State: SSSD, Response: SSSD, Error:SSSD + std::error::Error + From<std::io::Error>> ActorSinkStreamTrait for ActorSink<Actor, Message, State, Response, Error> {
    type Actor = Actor;
    type Message = Message;
    type State = State;
    type Response = Response;
    type Error = Error;
    fn sink_stream(actor_ref: Arc<ActorRef<Self::Actor, Self::Message, Self::State, Self::Response, Self::Error>>)
        -> (impl Sink<Self::Message, Error=Self::Error>, AsyncStream<Self::Response, impl Future<Output =()> + Sized>)
        where ActorRef<Self::Actor, Self::Message, Self::State, Self::Response, Self::Error>: ActorTrait {

        use tokio::sync::mpsc;

        let (tx, mut rx) = mpsc::channel::<Self::Response>(10000);

        let stream = async_stream::stream! {
            while let Some(item) = rx.recv().await {
                yield item;
            }
        };

        (
            ActorSinkAsk {
                actor_ref: actor_ref.clone(),
                tx
            },
            stream
        )
    }
}

