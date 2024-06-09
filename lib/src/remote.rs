use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Arc;
use futures::lock::Mutex;
use tokio::net::{TcpListener, TcpStream};
use crate::{ActorRef, Handler};

#[derive(Debug)]
pub struct ActorServer<Actor, Message, State, Response, Error> {
    actor_ref: Mutex<Option<Arc<crate::ActorRef<Actor, Message, State, Response, Error>>>>,
}


impl<Actor: Handler<Actor, Message, State, Response, Error> + Debug + Send + Sync + 'static, Message: Debug + Send + Sync + 'static, State: Debug + Send + Sync + 'static,
    Response: Debug + Send + Sync + 'static, Error: std::error::Error + Debug + Send + Sync + From<std::io::Error> + 'static> ActorServer<Actor, Message, State, Response, Error> {
    pub async fn new(name: impl AsRef<str>, host: impl AsRef<str>, port: u16, actor: Arc<ActorRef<Actor,Message, State, Response, Error>>) -> Result<Arc<Self>, Error>
    {

        let address = format!("{}:{}", host.as_ref(), port);
        let listener = TcpListener::bind(address).await?;
        let handle = tokio::runtime::Handle::current();
        let handle_loop= handle.spawn(async move {
            loop {
                // TODO: remove unwrap and handle error
                let (mut socket, address) = listener.accept().await.unwrap();
                println!("Connection from {address:?}");
            }
        });


        Ok(Arc::new(Self {
            actor_ref: Mutex::new(Some(actor)),
        }))
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
}


impl<Actor: Handler<Actor, Message, State, Response, Error> + Debug + Send + Sync + 'static, Message: Debug + Send + Sync + 'static, State: Debug + Send + Sync + 'static,
    Response: Debug + Send + Sync + 'static, Error: std::error::Error + Debug + Send + Sync + From<std::io::Error> + 'static> crate::remote::ActorClient<Actor, Message, State, Response, Error> {
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
        }))
    }


    pub async fn ask(&self, mgs: Message) -> Result<Response, Error>
    {
        todo!()
    }

    pub async fn send(&self, msg: Message) -> Result<(), std::io::Error> {
        todo!()
    }

    pub async fn state(&self) -> Result<Arc<Mutex<State>>, std::io::Error> {
        todo!()
    }
    pub async fn stop(&self) -> Result<(), Error> {
        todo!()
    }


}
