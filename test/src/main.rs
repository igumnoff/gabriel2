mod echo;

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

    // _ = echo_ref.stop().await;

    // ctrl-c wait
    tokio::signal::ctrl_c().await?;
    Ok(())
}


#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use std::sync::{Arc};
    use std::time::Duration;
    use gabriel2::*;
    use async_trait::async_trait;

    use thiserror::Error;
    use crate::echo::{Echo, EchoError, Message, Response, State};

    #[derive(Debug)]
    pub struct UserActor;

    #[derive(Debug)]
    pub enum UserMessage {
        CreateAccount { account_id: u32, } ,
        #[allow(dead_code)]
        GetBalance { account_id: u32, },
        #[allow(dead_code)]
        MoveMoney { from_account_id: u32, to_account_id: u32, amount: u32 },
    }

    #[derive(Debug)]
    pub enum UserResponse {
        Balance { amount: u32, },
        #[allow(dead_code)]
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
        #[allow(dead_code)]
        Unknown,
        #[error("std::io::Error")]
        StdErr(#[from] std::io::Error),
    }

    #[async_trait]
    impl Handler<UserActor, UserMessage, UserState, UserResponse, UserError> for UserActor {

        async fn receive(&self, ctx: Arc<Context<UserActor, UserMessage, UserState, UserResponse, UserError>>) -> Result<UserResponse, UserError> {
            match ctx.mgs {
                UserMessage::GetBalance { account_id: _ } => {
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

        let user:Arc<ActorRef<UserActor, UserMessage, UserState, UserResponse, UserError>>  = ActorRef::new("user".to_string(),
           UserActor {}, UserState {name: "".to_string()}, 10000).await?;

        let _result1: UserResponse = user.ask(UserMessage::CreateAccount{ account_id: 0 }).await?;
        {
            let actor_state = user.state().await?;
            let state_lock = actor_state.lock().await;
            let _name_from_state = state_lock.name.clone();

        }
        let user_clone = user.clone();
        tokio::spawn(async move {
            let _ = user.send(UserMessage::CreateAccount { account_id: 1 }).await;
            let _ = user.send(UserMessage::CreateAccount { account_id: 2 }).await;
            let _ = user.send(UserMessage::CreateAccount { account_id: 3 }).await;
        });
        let _ = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            let _ = user_clone.stop().await;
        });
        tokio::time::sleep(Duration::from_millis(1000)).await;
        Ok(())
    }


    #[tokio::test]
    async fn test_remote() -> anyhow::Result<()> {
        use gabriel2::remote::*;
        let _ = env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("trace")).try_init();


        let state = State {
            counter: 0,
        };

        let echo_ref = ActorRef::new("echo".to_string(), crate::echo::Echo {}, state, 100000).await?;
        let echo_server = ActorServer::new("echo_server", "127.0.0.1", 9001, echo_ref).await?;
        let echo_client: Arc<ActorClient<Echo, Message, State, Response, EchoError >> = ActorClient::new("echo_client", "127.0.0.1", 9001).await?;

        let pong = echo_client.ask(Message::Ping).await?;
        log::info!("Ask");
        echo_client.send(Message::Ping).await?;
        log::info!("Ping");
        echo_client.send(Message::Ping).await?;
        log::info!("Ping");
        // sleep 1 second
        tokio::time::sleep(Duration::from_secs(1)).await;

        // _ = echo_client.stop().await;
        // _ = echo_server.stop().await;
        Ok(())
    }

}

