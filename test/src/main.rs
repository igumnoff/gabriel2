mod echo;

use gabriel2::*;
use echo::*;
#[tokio::main]
async fn main() -> Result<(), EchoError> {
    let state = State {
        counter: 0,
    };

    let echo_ref = ActorRef::new("echo", Echo{},  state, 100000).await?;

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

    use thiserror::Error;
    use crate::echo::{Echo, EchoError, Message, Response, State};


    // #[tokio::test]
    // async fn test_remote() -> anyhow::Result<()> {
    //     use gabriel2::remote::*;
    //     // let _ = env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("trace")).try_init();
    //
    //
    //     let state = State {
    //         counter: 0,
    //     };
    //
    //     let echo_ref = ActorRef::new("echo".to_string(), crate::echo::Echo {}, state, 100000).await?;
    //     let echo_server = ActorServer::new("echo_server", "127.0.0.1", 9001, echo_ref).await?;
    //     let echo_client: Arc<ActorClient<Echo, Message, State, Response, EchoError >> = ActorClient::new("echo_client", "127.0.0.1", 9001).await?;
    //     println!("Sent Ping");
    //     echo_client.send(Message::Ping).await?;
    //     println!("Sent Ping and ask response");
    //     let pong = echo_client.ask(Message::Ping).await?;
    //     println!("Got {:?}", pong);
    //     tokio::time::sleep(Duration::from_secs(1)).await;
    //     _ = echo_client.stop().await;
    //     tokio::time::sleep(Duration::from_secs(1)).await;
    //     _ = echo_server.stop().await;
    //     tokio::time::sleep(Duration::from_secs(1)).await;
    //     Ok(())
    // }


}

