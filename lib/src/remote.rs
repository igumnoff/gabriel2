use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Arc;
use bincode::{config, Decode, Encode};
use bincode::error::{DecodeError, EncodeError};
use futures::lock::Mutex;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::oneshot;
use tokio::sync::oneshot::Sender;
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
                                        let message_result: Result<RequestMessage, DecodeError> = deserialize_command(&buf[0..n]);
                                        match message_result {
                                            Ok(request_message) => {
                                                match request_message.command {
                                                    Command::Send => {
                                                        // log::info!("Payload: {:?}", request_message.payload);
                                                        let message_result:  Result<Message, DecodeError> = deserialize(&request_message.payload[..]);
                                                        match message_result {
                                                            Ok(message) => {
                                                                log::info!("<{name}> Received message: {message:?}");
                                                                let actor_ref = actor_server_clone2.actor_ref.lock().await;
                                                                let send_result = actor_ref.as_ref().unwrap().send(message).await;
                                                                if send_result.is_err() {
                                                                    log::error!("<{name}> Error sending message: {:?}", send_result.err());
                                                                    break
                                                                }
                                                            }
                                                            Err(err) => {
                                                                log::error!("<{name}> Error deserializing message (payload): {:?}", err);
                                                                break;
                                                            }
                                                        }
                                                    }
                                                    Command::Ask => {
                                                        let message_result:  Result<Message, DecodeError> = deserialize(&request_message.payload[..]);
                                                        match message_result {
                                                            Ok(message) => {
                                                                log::info!("<{name}> Received message: {message:?}");
                                                                let actor_ref = actor_server_clone2.actor_ref.lock().await.clone().unwrap();
                                                                let send_result = actor_ref.ask(message).await;
                                                                if send_result.is_err() {
                                                                    log::error!("<{name}> Error sending message: {:?}", send_result.err());
                                                                    break
                                                                }
                                                            }
                                                            Err(err) => {
                                                                log::error!("<{name}> Error deserializing message (payload): {:?}", err);
                                                                break;
                                                            }
                                                        }
                                                    }
                                                    Command::State => {}
                                                    Command::Stop => {}
                                                }
                                            }
                                            Err(err) => {
                                                log::error!("<{name}> Error deserializing message (command): {:?}", err);
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
    counter: Mutex<u64>,
    name: String,
    promise: Mutex<HashMap<u64, Sender<Result<Response, Error>>>>,
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
            counter: Mutex::new(0),
            promise: Mutex::new(HashMap::new()),
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


    pub async fn ask(&self, msg: Message) -> Result<Response, Error>
    {
        let (sender, receiver) = oneshot::channel();
        {

            let counter = {
                let mut counter = self.counter.lock().await;
                *counter += 1;
                *counter
            };
            let name = &self.name;
            let mut stream = self.write_half.lock().await;
            let data = serialize(counter, Command::Ask, Some(&msg)).map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
            stream.write_all(&data[..]).await?;
            log::info!("<{name}> Sent message");
            self.promise.lock().await.insert(counter, sender);
        }
        let r = receiver.await;
        match r {
            Ok(res) => { res }
            Err(_) => {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "Err").into());
            }
        }
    }

    pub async fn send(&self, msg: Message) -> Result<(), std::io::Error> {
        let counter = {
            let mut counter = self.counter.lock().await;
            *counter += 1;
            *counter
        };
        let name = &self.name;
        let mut stream = self.write_half.lock().await;
        let data = serialize(counter, Command::Send, Some(&msg)).map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
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

fn serialize<T: Encode + Decode>(id: u64, command: Command, payload: Option<&T>) -> Result<Vec<u8>, EncodeError> {
    let config = config::standard();
    let encoded: Vec<u8> = if payload.is_some() {
        bincode::encode_to_vec(payload.unwrap(), config)?
    } else {
        vec![]
    };
    // log::info!("Serialized: {:?}", encoded);
    let request_message = RequestMessage {
        id,
        command,
        payload: encoded,
    };
    let request_message_encoded = bincode::encode_to_vec(&request_message, config)?;
    Ok(request_message_encoded)
}

fn deserialize_command(data: &[u8]) -> Result<RequestMessage, DecodeError> {
    let config = config::standard();
    let (request_message, _): (RequestMessage, _) = bincode::decode_from_slice(data, config)?;
    Ok(request_message)
}

fn deserialize<T: Encode + Decode>(data: &[u8]) -> Result<T, DecodeError> {
    // log::info!("Deserializing: {:?}", data);
    let config = config::standard();
    let (payload, _): (T, _) = bincode::decode_from_slice(data, config)?;
    Ok(payload)
}