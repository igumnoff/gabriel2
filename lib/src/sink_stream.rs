use std::pin::Pin;
use std::task::{Context, Poll};
use futures::sink::Sink;
use futures::stream::Stream;
use crate::{ActorTrait, Handler};
use crate::SSSD;
use crate::ActorRef;
pub trait ActorSinkTrait {
    type Actor: SSSD;
    type Message: SSSD;
    type State: SSSD;
    type Response: SSSD;
    type Error: SSSD + std::error::Error + From<std::io::Error>;
    fn new_sink(actor_ref: ActorRef<Self::Actor, Self::Message, Self::State, Self::Response, Self::Error>)
        -> impl Sink<Self::Message, Error=Self::Error>
        where ActorRef<Self::Actor, Self::Message, Self::State, Self::Response, Self::Error>: ActorTrait;
}


// impl <Actor:SSSD, Message:  SSSD, State: SSSD, Response: SSSD,
//     Error: SSSD + std::error::Error + From<std::io::Error>> ActorSinkTrait for ActorSink<Actor, Message, State, Response, Error> {
//     type Actor = Actor;
//     type Message = Message;
//     type State = State;
//     type Response = Response;
//     type Error = Error;
//     fn new_sink(actor_ref: ActorRef<Self::Actor, Self::Message, Self::State, Self::Response, Self::Error>) -> impl Sink<Self::Message, Error=Self::Error>
//         where ActorRef<Self::Actor, Self::Message, Self::State, Self::Response, Self::Error>: ActorTrait
//     {
//         ActorSink {
//             actor_ref
//         }
//     }
// }

struct ActorSink<Actor, Message, State, Response, Error>
    where ActorRef<Actor, Message, State, Response, Error>: ActorTrait,
    Error: SSSD + std::error::Error + From<std::io::Error>,
{
    actor_ref: ActorRef<Actor, Message, State, Response, Error>
}


impl <Actor:Handler<Actor = Actor, State = State, Message = Message, Response = Response, Error = Error> + SSSD, Message: SSSD,
    State: SSSD, Response: SSSD, Error: SSSD + std::error::Error + From<std::io::Error>> Sink<Message> for ActorSink<Actor, Message, State, Response, Error> {
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Message) -> Result<(), Error> {
        let handle = tokio::runtime::Handle::current();
        handle.block_on(async {
            self.actor_ref.send(item).await;
        });
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}
//
// pub trait ActorSinkStream {
//     type Actor:  Handler + SSSD;
//     type Message: SSSD;
//     type State: SSSD;
//     type Response: SSSD;
//     type Error: SSSD + std::error::Error + From<std::io::Error>;
//     fn new_sink_stream(&self) -> (impl Sink<Self::Message, Error=Self::Error>, impl Stream<Item=Self::Response>);
// }

