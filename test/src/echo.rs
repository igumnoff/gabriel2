use std::sync::Arc;
use gabriel2::*;
use thiserror::Error;

use async_trait::async_trait;


#[derive(Debug)]
pub struct Echo;

#[derive(Debug)]
pub enum Message {
    Ping,
}

#[derive(Debug)]
pub enum Response {
    Pong {counter: u32},
}

#[derive(Debug,Clone)]
pub struct State {
    pub counter: u32,
}

#[derive(Debug, Error)]
pub enum EchoError {
    #[error("unknown error")]
    Unknown,
    #[error("std::io::Error")]
    StdErr(#[from] std::io::Error),
}

#[async_trait]
impl Handler<Echo, Message, State, Response, EchoError> for Echo {
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