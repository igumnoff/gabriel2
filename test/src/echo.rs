use std::sync::Arc;
use gabriel2::*;

use bincode::{Decode, Encode};
use derive_more::{Display, Error};


#[derive(Debug)]
pub struct Echo;

#[derive(Debug, Encode, Decode)]
pub enum Message {
    Ping,
}

#[derive(Debug, Encode, Decode)]
pub enum Response {
    Pong {counter: u32},
}

#[derive(Debug,Clone, Encode, Decode)]
pub struct State {
    pub counter: u32,
}

#[derive(Debug, Display, Error,  Encode, Decode)]
pub enum EchoError {
    #[display(fmt = "Unknown error")]
    Unknown,
}

impl From<std::io::Error> for EchoError {
    fn from(_err: std::io::Error) -> Self {
        EchoError::Unknown
    }
}

impl Handler for Echo {
    type Actor = Echo;
    type Message = Message;
    type State = State;
    type Response = Response;
    type Error = EchoError;

    async fn receive(&self, ctx: Arc<Context<Echo, Message, State, Response, EchoError>>) -> Result<Response, EchoError> {
        match ctx.mgs {
            Message::Ping => {
                println!("Received Ping");
                let mut state_lock = ctx.state.lock().await;
                state_lock.counter += 1;
                if state_lock.counter > 10 {
                    Err(EchoError::Unknown)
                } else {
                    Ok(Response::Pong{counter: state_lock.counter})
                }
            }
        }
    }
}