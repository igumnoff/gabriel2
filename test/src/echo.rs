use std::sync::Arc;
use gabriel2::*;

use bincode::{Decode, Encode};
use derive_more::{Display, Error};


#[derive(Debug, Encode, Decode)]
pub struct EchoActor;

#[derive(Debug, Encode, Decode)]
pub enum EchoMessage {
    Ping,
}

#[derive(Debug, Encode, Decode)]
pub enum EchoResponse {
    Pong {counter: u32},
}

#[derive(Debug,Clone, Encode, Decode)]
pub struct EchoState {
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

impl Handler for EchoActor {
    type Actor = EchoActor;
    type Message = EchoMessage;
    type State = EchoState;
    type Response = EchoResponse;
    type Error = EchoError;

    async fn receive(&self, ctx: Arc<Context<Self::Actor, Self::Message, Self::State, Self::Response, Self::Error>>) -> Result<EchoResponse, EchoError> {
        match ctx.mgs {
            EchoMessage::Ping => {
                println!("Received Ping");
                let mut state_lock = ctx.state.lock().await;
                state_lock.counter += 1;
                if state_lock.counter > 10 {
                    Err(EchoError::Unknown)
                } else {
                    Ok(EchoResponse::Pong{counter: state_lock.counter})
                }
            }
        }
    }
}