# Gabriel2

![Gabriel2](https://github.com/igumnoff/gabriel2/raw/HEAD/logo.png)

Gabriel2: Indeed, an actor library based on Tokio, written in Rust

## Features

- [x] Async for sending messages
- [x] Async for messages processing in actor
- [x] Support messaging like send and forget 
- [x] Support messaging like send and wait response
- [x] Mutable state of actor
- [x] Self reference in actor from context
- [x] Actor lifecycle (pre_start, pre_stop)
- [x] Sink to actor
- [x] Stream from actor
- [x] Remote Actor
- [x] Event Bus
- [x] Load Balancer


## Usage

Cargo.toml

```toml
[dependencies]
gabriel2 = { version = "1.5.0", features = ["remote", "sink-stream", "broadcast", "balancer"] }
```

echo.rs

```rust
use std::sync::Arc;
use gabriel2::*;

use bincode::{Decode, Encode};
use derive_more::{Display, Error};


#[derive(Debug)]
pub struct EchoActor;

#[derive(Debug)]
pub enum EchoMessage {
    Ping,
}

#[derive(Debug)]
pub enum EchoResponse {
    Pong {counter: u32},
}

#[derive(Debug,Clone)]
pub struct EchoState {
    pub counter: u32,
}

#[derive(Debug, Display, Error)]
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
```

main.rs

```rust
#[tokio::main]
async fn main() -> Result<(), EchoError> {
    let state = EchoState {
        counter: 0,
    };

    let echo_ref = ActorRef::new("echo", EchoActor {}, state, 100000).await?;

    println!("Sent Ping");
    echo_ref.send(EchoMessage::Ping).await?;

    println!("Sent Ping and ask response");
    let pong = echo_ref.ask(EchoMessage::Ping).await?;
    println!("Got {:?}", pong);

    _ = echo_ref.stop().await;
    Ok(())
}
```

Example output:

```text 
Sent Ping
Sent Ping and ask response
Received Ping
Received Ping
Got Pong { counter: 2 }
```

Example sources: https://github.com/igumnoff/gabriel2/tree/main/test


## Sink

```rust
#[tokio::main]
async fn main() -> Result<(), EchoError> {
    let state = EchoState {
        counter: 0,
    };

    let echo_ref = ActorRef::new("echo", crate::echo::EchoActor {}, state, 100000).await?;
    let echo_sink = ActorSink::sink(echo_ref.clone());
    let message_stream = futures::stream::iter(vec![EchoMessage::Ping, EchoMessage::Ping, EchoMessage::Ping]).map(Ok);
    _ = message_stream.forward(echo_sink).await;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
     Ok(())
}
```

Example output:
```
Received Ping
Received Ping
Received Ping
```

### Stream

```rust
#[tokio::main]
async fn main() -> Result<(), EchoError> {
    let state = EchoState {
        counter: 0,
    };

    let echo_ref = ActorRef::new("echo", crate::echo::EchoActor {}, state, 100000).await?;
    let (echo_sink, echo_stream) = ActorSink::sink_stream(echo_ref.clone());
    let message_stream = futures::stream::iter(vec![EchoMessage::Ping, EchoMessage::Ping, EchoMessage::Ping]).map(Ok);
    _ = message_stream.forward(echo_sink).await;
    echo_stream.for_each(|message| async move {
        println!("Got {:?}", message.unwrap());
    }).await;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    Ok(())
}
```
Example output:

```
Received Ping
Received Ping
Received Ping
Got Pong { counter: 1 }
Got Pong { counter: 2 }
Got Pong { counter: 3 }
```

## Remote 

Preparations for remote:

Add Encode, Decode from "bincode" to derive(..) for EchoActor, EchoMessage, EchoResponse, EchoState and EchoError

Remote version:

```rust
#[tokio::main]
async fn main() -> Result<(), EchoError> {
    let state = EchoState {
        counter: 0,
    };

    let echo_ref = ActorRef::new("echo", crate::echo::EchoActor {}, state, 100000).await?;
    let echo_server = ActorServer::new("echo_server", "127.0.0.1", 9001, echo_ref).await?;
    let echo_client: Arc<ActorClient<EchoActor, EchoMessage, EchoState, EchoResponse, EchoError >> = ActorClient::new("echo_client", "127.0.0.1", 9001).await?;

    println!("Sent Ping");
    echo_client.send(EchoMessage::Ping).await?;

    println!("Sent Ping and ask response");
    let pong = echo_client.ask(EchoMessage::Ping).await?;
    println!("Got {:?}", pong);

    _ = echo_client.stop().await;
    _ = echo_server.stop().await;
    Ok(())

}
```

### Event Bus

```rust
#[tokio::main]
async fn main() -> Result<(), EchoError> {
    let state = EchoState {
        counter: 0,
    };

    #[derive(Debug, Copy, Clone)]
    enum EventElement {
        Fire,
        Water
    }

    let echo_ref = ActorRef::new("echo", crate::echo::EchoActor {}, state, 100000).await?;

    let event_bus: Arc<EventBus<EventElement>> = Arc::new(EventBus::new());

    let subscriber_id = event_bus.subscribe(move |event: EventElement| {
        async move {
            match event {
                EventElement::Fire => {
                    let _ = echo_ref.send(EchoMessage::Ping).await;
                    ()
                },
                _ => ()
            }
        }}).await;

    event_bus.publish(EventElement::Fire).await;

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    event_bus.unsubscribe(subscriber_id).await;

    Ok(())
}
``` 

### Load Balancer

```rust
#[tokio::main]
async fn main() -> Result<(), EchoError> {
    let echo_load_balancer: Arc<LoadBalancer<EchoActor, EchoMessage, EchoState, EchoResponse, EchoError>> =
        LoadBalancer::new("echo_load_balancer", 10, |id: usize| {
            Box::pin(async move {
                let user: Arc<
                    ActorRef<EchoActor, EchoMessage, EchoState, EchoResponse, EchoError>,
                > = ActorRef::new(
                    format!("echo-{}", id),
                    EchoActor {},
                    EchoState { counter: 0 },
                    10000,
                )
                    .await?;
                Ok(user)
            })
        })
            .await
            .unwrap();

    for _ in 0..30 {
        echo_load_balancer.send(EchoMessage::Ping).await?;
    }

    Ok(())
}
``` 


## Contributing
I would love to see contributions from the community. If you experience bugs, feel free to open an issue. If you would like to implement a new feature or bug fix, please follow the steps:
1. Read "[Contributor License Agreement (CLA)](https://github.com/igumnoff/gabriel2/blob/main/CLA)"
2. Contact with me via telegram @ievkz or discord @igumnovnsk
3. Confirm e-mail invitation in repository
4. Do "git clone" (You don't need to fork!)
5. Create branch with your assigned issue
6. Create pull request to main branch
