use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Arc;
use futures::lock::Mutex;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::{ActorRef, Handler};

#[derive(Debug)]
pub struct ActorServer<Actor, Message, State, Response, Error> {
    actor_ref: Mutex<Option<Arc<crate::ActorRef<Actor, Message, State, Response, Error>>>>,
    failed: Mutex<bool>,
}


impl<Actor: Handler<Actor, Message, State, Response, Error> + Debug + Send + Sync + 'static, Message: Debug + Send + Sync + 'static, State: Debug + Send + Sync + 'static,
    Response: Debug + Send + Sync + 'static, Error: std::error::Error + Debug + Send + Sync + From<std::io::Error> + 'static>
ActorServer<Actor, Message, State, Response, Error> {
    pub async fn new(name: impl AsRef<str>, host: impl AsRef<str>, port: u16, actor: Arc<ActorRef<Actor,Message, State, Response, Error>>) -> Result<Arc<Self>, Error>
    {

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
                        handle.spawn(async move {
                            log::info!("Connection opened from {address:?}");
                            let mut buf = vec![0; 1024];
                            loop {
                                let n = socket
                                    .read(&mut buf)
                                    .await
                                    .expect("failed to read data from socket");
                                let message = String::from_utf8_lossy(&buf[0..n]);
                                log::info!("Received message: {message}");
                                if n == 0 {
                                    return;
                                }

                                // socket
                                //     .write_all(&buf[0..n])
                                //     .await
                                //     .expect("failed to write data to socket");
                            }
                            log::info!("Connection closed from {address:?}");
                        });

                    }
                    Err(err) => {
                        log::error!("Error accepting connection: {:?}", err);
                        let mut failed = actor_server_clone.failed.lock().await;
                        *failed = true;
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
    tcp_stream: Mutex<TcpStream>,
}


impl<Actor: Handler<Actor, Message, State, Response, Error> + Debug + Send + Sync + 'static, Message: Debug + Send + Sync + 'static, State: Debug + Send + Sync + 'static,
    Response: Debug + Send + Sync + 'static, Error: std::error::Error + Debug + Send + Sync + From<std::io::Error> + 'static>
ActorClient<Actor, Message, State, Response, Error> {
    pub async fn new(name: impl AsRef<str>, host: impl AsRef<str>, port: u16) -> Result<Arc<Self>, Error>
    {
        let address = format!("{}:{}", host.as_ref(), port);

        let mut stream = TcpStream::connect(address).await?;
        println!("Connected to the server!");

        Ok(Arc::new(Self {
            _marker_actor: PhantomData,
            _marker_message: PhantomData,
            _marker_state: PhantomData,
            _marker_response: PhantomData,
            _marker_error: PhantomData,
            tcp_stream: Mutex::new(stream),
        }))
    }


    pub async fn ask(&self, mgs: Message) -> Result<Response, Error>
    {
        todo!()
    }

    pub async fn send(&self, msg: Message) -> Result<(), std::io::Error> {
        let mut stream = self.tcp_stream.lock().await;
        stream.write_all(b"ping").await?;
        Ok(())
    }

    pub async fn state(&self) -> Result<Arc<Mutex<State>>, std::io::Error> {
        todo!()
    }
    pub async fn stop(&self) -> Result<(), Error> {
        todo!()
    }


}
