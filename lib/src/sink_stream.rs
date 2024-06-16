use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use futures::sink::Sink;
use futures::stream::Stream;
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
            -> (impl Sink<Self::Message, Error=Self::Error>, impl Stream<Item = Self::Message>)
        where ActorRef<Self::Actor, Self::Message, Self::State, Self::Response, Self::Error>: ActorTrait;
}

pub struct ActorSinkStream<Actor, Message, State, Response, Error>
    where ActorRef<Actor, Message, State, Response, Error>: ActorTrait{
    actor_ref: Arc<ActorRef<Actor, Message, State, Response, Error>>
}

impl <Actor: Handler<Actor = Actor, Message = Message, State = State,Response = Response, Error = Error> + SSSD,
    Message: SSSD, State: SSSD, Response: SSSD, Error:SSSD + std::error::Error + From<std::io::Error>> ActorSinkStreamTrait for ActorSink<Actor, Message, State, Response, Error> {
    type Actor = Actor;
    type Message = Message;
    type State = State;
    type Response = Response;
    type Error = Error;
    fn sink_stream(actor_ref: Arc<ActorRef<Self::Actor, Self::Message, Self::State, Self::Response, Self::Error>>) -> (impl Sink<Self::Message, Error=Self::Error>, impl Stream<Item=Self::Message>)
        where ActorRef<Self::Actor, Self::Message, Self::State, Self::Response, Self::Error>: ActorTrait {

        (
            ActorSink {
                actor_ref: actor_ref.clone()
            },
            ActorSinkStream {
                actor_ref: actor_ref.clone()
            }
        )
    }
}

impl <Actor:Handler<Actor = Actor, State = State, Message = Message, Response = Response, Error = Error> + SSSD,
    Message: SSSD, State: SSSD, Response: SSSD, Error: SSSD + std::error::Error + From<std::io::Error>> Stream for ActorSinkStream<Actor, Message, State, Response, Error> {
    type Item = Message;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        todo!()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        todo!()
    }
}
