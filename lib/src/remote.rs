use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Arc;
use bincode::{config, Decode, Encode};
use bincode::error::{DecodeError, EncodeError};
use futures::lock::Mutex;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::{ActorRef, Handler};

#[derive(Debug)]
pub struct ActorServer<Actor, Message, State, Response, Error> {
    actor_ref: Mutex<Option<Arc<crate::ActorRef<Actor, Message, State, Response, Error>>>>,
    failed: Mutex<bool>,
}


impl<Actor: Handler<Actor, Message, State, Response, Error> + Debug + Encode + Decode  + Send + Sync + 'static, Message: Debug + Encode + Decode  + Send + Sync + 'static, State: Debug + Encode + Decode  + Send + Sync + 'static,
    Response: Debug + Encode + Decode  + Send + Sync + 'static, Error: std::error::Error + Debug + Encode + Decode  + Send + Sync + From<std::io::Error> + 'static>
ActorServer<Actor, Message, State, Response, Error> {
    pub async fn new(name: impl AsRef<str>, host: impl AsRef<str>, port: u16, actor: Arc<ActorRef<Actor,Message, State, Response, Error>>) -> Result<Arc<Self>, Error>
    {
        let name = name.as_ref().to_string();
        let address = format!("{}:{}", host.as_ref(), port);
        let listener = TcpListener::bind(address).await?;
        let handle = tokio::runtime::Handle::current();
        let actor_server = Arc::new(Self {
            actor_ref: Mutex::new(Some(actor)),
            failed: Mutex::new(false),
        });
        let actor_server_clone = actor_server.clone();
        let _handle_loop = handle.spawn(async move {
            let handle = tokio::runtime::Handle::current();
            loop {
                let accept_result = listener.accept().await;
                match accept_result {
                    Ok((mut socket, address)) => {
                        let name = name.clone();
                        let actor_server_clone2 = actor_server_clone.clone();
                        handle.spawn(async move {
                            log::info!("<{name}> Connection opened from {address:?}");
                            let mut buf = vec![0; 1024];
                            loop {
                                match socket.read(&mut buf).await {
                                    Ok(n) => {
                                        if n == 0 {
                                            break;
                                        }
                                        let message_result: Result<(u64, crate::remote::Command, Message), std::io::Error> = deserialize(&buf[0..n]).map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err));
                                        match message_result {
                                            Ok((id, command, message)) => {
                                                log::info!("<{name}> Received message: {message:#?}");
                                                let actor_ref = actor_server_clone2.actor_ref.lock().await;
                                                actor_ref.as_ref().unwrap().send(message).await.unwrap();
                                            }
                                            Err(err) => {
                                                log::error!("<{name}> Error deserializing message: {:?}", err);
                                                break
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        log::error!("<{name}> Error reading from socket: {:?}", err);
                                        break
                                    }
                                }
                            }
                            log::info!("<{name}> Connection closed from {address:?}");
                        });
                    }
                    Err(err) => {
                        log::error!("<{name}> Error accepting connection: {:?}", err);
                        let mut failed = actor_server_clone.failed.lock().await;
                        *failed = true;
                        break
                    }
                }
            }
        });



        Ok(actor_server.clone())
    }
    pub async fn stop(&self) -> Result<(), Error> {
        todo!()
    }
}


#[derive(Debug)]
pub struct ActorClient<Actor, Message, State, Response, Error> {
    _marker_actor: PhantomData<Actor>,
    _marker_message: PhantomData<Message>,
    _marker_state: PhantomData<State>,
    _marker_response: PhantomData<Response>,
    _marker_error: PhantomData<Error>,
    read_half: Mutex<tokio::io::ReadHalf<TcpStream>>,
    write_half: Mutex<tokio::io::WriteHalf<TcpStream>>,
    name: String,
}


impl<Actor: Handler<Actor, Message, State, Response, Error> + Debug + Encode + Decode + Send + Sync + 'static, Message: Debug + Encode + Decode + Send + Sync + 'static, State: Debug + Encode + Decode + Send + Sync + 'static,
    Response: Debug + Encode + Decode + Send + Sync + 'static, Error: std::error::Error + Debug + Encode + Decode + Send + Sync + From<std::io::Error> + 'static>
ActorClient<Actor, Message, State, Response, Error> {
    pub async fn new(name: impl AsRef<str>, host: impl AsRef<str>, port: u16) -> Result<Arc<Self>, Error>
    {
        let name = name.as_ref().to_string();
        let address = format!("{}:{}", host.as_ref(), port);
        let stream = TcpStream::connect(address.clone()).await?;
        log::info!("<{name}> Connected to ActorServer at {address}");

        let (read_half, write_half) = tokio::io::split(stream);

        let actor = Arc::new(Self {
            _marker_actor: PhantomData,
            _marker_message: PhantomData,
            _marker_state: PhantomData,
            _marker_response: PhantomData,
            _marker_error: PhantomData,
            read_half: Mutex::new(read_half),
            write_half: Mutex::new(write_half),
            name: name.clone(),
        });

        let handle = tokio::runtime::Handle::current();
        let actor_clone = actor.clone();
        let _handle_loop = handle.spawn(async move {
            let mut buf = vec![0; 1024];
            loop {
                let mut stream = actor_clone.read_half.lock().await;
                match stream.read(&mut buf).await {
                    Ok(n) => {
                        if n == 0 {
                            break;
                        }
                        let message = String::from_utf8_lossy(&buf[0..n]);
                        log::info!("<{name}> Received message: {message}");
                    }
                    Err(err) => {
                        log::error!("<{name}> Error reading from socket: {:?}", err);
                        break
                    }
                }

            }
            log::info!("<{name}> Connection closed");
        });
        Ok(actor)
    }


    pub async fn ask(&self, mgs: Message) -> Result<Response, Error>
    {
        todo!()
    }

    pub async fn send(&self, msg: Message) -> Result<(), std::io::Error> {
        let name = &self.name;
        let mut stream = self.write_half.lock().await;
        let data = serialize(0, Command::Send, &msg).map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
        stream.write_all(&data[..]).await?;
        log::info!("<{name}> Sent message");
        Ok(())
    }

    pub async fn state(&self) -> Result<Arc<Mutex<State>>, std::io::Error> {
        todo!()
    }
    pub async fn stop(&self) -> Result<(), Error> {
        todo!()
    }


}

#[derive(Encode, Decode, PartialEq, Debug)]
enum Command {
    Send,
    Ask,
    State,
    Stop,
}

#[derive(Encode, Decode, PartialEq, Debug)]
struct RequestMessage {
    id: u64,
    command: Command,
    payload: Vec<u8>,
}

fn serialize<T: Encode + Decode>(id: u64, command: Command, payload: &T) -> Result<Vec<u8>, EncodeError> {
    let config = config::standard();
    let encoded: Vec<u8> = bincode::encode_to_vec(payload, config)?;
    let request_message = RequestMessage {
        id,
        command,
        payload: encoded,
    };
    let request_message_encoded = bincode::encode_to_vec(&request_message, config)?;
    Ok(request_message_encoded)
}

fn deserialize<T: Encode + Decode>(data: &[u8]) -> Result<(u64, Command, T), DecodeError> {
    let config = config::standard();
    let (request_message, _): (RequestMessage, _) = bincode::decode_from_slice(data, config)?;
    let (payload, _): (T, _) = bincode::decode_from_slice(&request_message.payload, config)?;
    Ok((request_message.id, request_message.command, payload))
}