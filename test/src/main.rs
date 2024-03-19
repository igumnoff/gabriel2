mod echo;

use std::time::Duration;
use gabriel2::*;
use echo::*;
#[tokio::main]
async fn main() -> Result<(), EchoError> {
    let state = State {
        counter: 0,
    };

    let echo_ref = ActorRef::new("echo".to_string(), Echo{},  state, 100000).await?;

    println!("Sent Ping");
    echo_ref.send(Message::Ping).await?;

    println!("Sent Ping and ask response");
    let pong = echo_ref.ask(Message::Ping).await?;
    println!("Got {:?}", pong);

    _ = echo_ref.stop();
    tokio::time::sleep(Duration::from_millis(1000)).await;
    Ok(())
}


#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use std::sync::{Arc};
    use std::time::Duration;
    use actorlib::*;
    use async_trait::async_trait;

    use thiserror::Error;

    #[derive(Debug)]
    pub struct UserActor;

    #[derive(Debug)]
    pub enum UserMessage {
        CreateAccount { account_id: u32, } ,
        GetBalance { account_id: u32, },
        MoveMoney { from_account_id: u32, to_account_id: u32, amount: u32 },
    }

    #[derive(Debug)]
    pub enum UserResponse {
        Balance { amount: u32, },
        AccountCreated { account_id: u32, },
        Ok,
    }
    #[derive(Debug,Clone)]
    pub struct UserState {
        pub name: String,
    }

    #[derive(Error, Debug)]
    pub enum UserError {
        #[error("unknown error")]
        Unknown,
        #[error("std::io::Error")]
        StdErr(#[from] std::io::Error),
    }

    #[async_trait]
    impl Handler<UserActor, UserMessage, UserState, UserResponse, UserError> for UserActor {

        async fn receive(&self, ctx: Arc<Context<UserActor, UserMessage, UserState, UserResponse, UserError>>) -> Result<UserResponse, UserError> {
            match ctx.mgs {
                UserMessage::GetBalance { .. } => {
                    Ok(UserResponse::Balance { amount: 100 })
                }
                _ => {
                    log::debug!("UserActor received {:?}", ctx.mgs);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    Ok(UserResponse::Ok)
                }
            }
        }
    }

    #[tokio::test]
    async fn test_2() -> Result<(), UserError> {
        let _ = env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("trace")).try_init();

        let mut user:Arc<ActorRef<UserActor, UserMessage, UserState, UserResponse, UserError>>  = ActorRef::new("user".to_string(),
           UserActor {}, UserState {name: "".to_string()}, 10000).await?;

        let result1: UserResponse = user.ask(UserMessage::CreateAccount{ account_id: 0 }).await?;
        {
            let actor_state = user.state().await?;
            let state_lock = actor_state.lock().await;
            let name_from_state = state_lock.name.clone();

        }
        let user_clone = user.clone();
        tokio::spawn(async move {
            let _ = user.send(UserMessage::CreateAccount { account_id: 1 }).await;
            let _ = user.send(UserMessage::CreateAccount { account_id: 2 }).await;
            let _ = user.send(UserMessage::CreateAccount { account_id: 3 }).await;
        });
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            user_clone.stop().await;
        });
        tokio::time::sleep(Duration::from_millis(1000)).await;
        Ok(())
    }
}

