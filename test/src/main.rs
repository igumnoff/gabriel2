mod echo;

use gabriel2::*;
use echo::*;

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


#[cfg(test)]
mod tests {
    use std::sync::{Arc};
    use futures::{StreamExt};
    use gabriel2::*;
    use gabriel2::sink_stream::{ActorSink, ActorSinkStreamTrait};
    use gabriel2::sink_stream::ActorSinkTrait;
    use crate::echo::{EchoActor, EchoError, EchoMessage, EchoResponse, EchoState};

    #[tokio::test]
    async fn test_remote() -> anyhow::Result<()> {
        use gabriel2::remote::*;
        // let _ = env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("trace")).try_init();

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

    #[tokio::test]
    async fn test_sink() -> anyhow::Result<()> {
        // let _ = env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("trace")).try_init();

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

    #[tokio::test]
    async fn test_sink_stream() -> anyhow::Result<()> {
        // let _ = env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("trace")).try_init();

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


    use crate::broadcast::*;

    #[derive(Debug, Copy, Clone)]
    enum EventElement {
        Fire,
        Water
    }

    impl Event for EventElement {}


    #[tokio::test]
    async fn check_subscription() -> anyhow::Result<()> {
        use crate::broadcast::*;
        let event_bus: Arc<EventBus<EventElement>> = Arc::new(EventBus::new());
        let subscriber_id = event_bus.subscribe(|event| async move {
            let otp = match event {
                EventElement::Fire => "FIIIRE!",
                EventElement::Water => "WAAATER!"
            };
            println!("{}", otp);
        }).await;

        event_bus.publish(EventElement::Water).await?;
        event_bus.publish(EventElement::Fire).await?;
        event_bus.publish(EventElement::Water).await?;
        let subscriber_id = event_bus.subscribe(|event| async move {
            let otp = match event {
                EventElement::Fire => "1",
                EventElement::Water => "2"
            };
            println!("{}", otp);
        }).await;

        event_bus.publish(EventElement::Fire).await?;
        event_bus.publish(EventElement::Water).await?; // FIXME: subscribers will see only last publish.
        // tokio::time::sleep(tokio::time::Duration::from_millis(10000)).await;
        Ok(())
    }

    // #[tokio::test]
    // async fn actor_event_bus_test() {
    //     use crate::broadcast::*;
    //     let event_bus: Arc<EventBus<EventElement>> = Arc::new(EventBus::new());
    //     let state = EchoState {
    //         counter: 0,
    //     };
    //     let echo_ref = Arc::new(ActorRef::new("echo".to_string(), crate::echo::EchoActor {}, state, 100000).await.unwrap());

    //     let e = echo_ref.clone();

    //     let subscriber_id = event_bus.subscribe(|event| async move {
    //         match event {
    //             EventElement::Fire => {
    //                 e.send(EchoMessage::Ping).await;
    //             },
    //             _ => ()
    //         }
    //     }).await;

    //     event_bus.publish(EventElement::Fire).await;
    //     event_bus.publish(EventElement::Fire).await;
    //     event_bus.publish(EventElement::Water).await;
    //     event_bus.publish(EventElement::Fire).await;
    //     event_bus.unsubscribe(subscriber_id).await; // <- FIXME: This thing executing not conseqentualy, and i dunno is it bug or feature xd.


    //     let state = if let Ok(EchoResponse::Pong {counter}) = echo_ref.ask(EchoMessage::Ping).await {
    //         counter
    //     } else {0};

    //     assert_eq!(state, 4) // why is state = 4 ;-;
    // }

}

