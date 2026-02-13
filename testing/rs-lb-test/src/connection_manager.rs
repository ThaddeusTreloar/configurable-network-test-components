use std::{marker::PhantomData, net::SocketAddr};

use bb8::ManageConnection;
use hyper::{body::Body, client::conn::http1::SendRequest};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

#[derive(Debug, thiserror::Error)]
pub enum ConnectionManagerError {
    #[error(transparent)]
    HyperError(hyper::Error),
    #[error("Connection is closed.")]
    ConnectionClosed,
    #[error(transparent)]
    UnableToConnect(std::io::Error),
}

#[derive(Debug, Clone)]
pub struct ConnectionManager<T> {
    addr: SocketAddr,
    _phantom_type: PhantomData<T>,
}

impl<T> ConnectionManager<T> {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            _phantom_type: Default::default(),
        }
    }
}

impl<T> ManageConnection for ConnectionManager<T>
where
    T: Send + Sync + Body + 'static,
    T::Data: Send,
    T::Error: Into<Box<dyn serde::ser::StdError + Send + Sync>>,
{
    type Connection = SendRequest<T>;
    type Error = ConnectionManagerError;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let stream = TcpStream::connect(self.addr)
            .await
            .map_err(ConnectionManagerError::UnableToConnect)?;

        let io = TokioIo::new(stream);

        let (sender, conn) = hyper::client::conn::http1::Builder::new()
            .handshake::<_, T>(io)
            .await
            .map_err(ConnectionManagerError::HyperError)?;

        tokio::task::spawn(async move {
            match conn.await {
                Ok(_) => (),
                Result::Err(err) => println!("Connection failed: {:?}", err),
            }
        });

        Ok(sender)
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        if self.has_broken(conn) {
            Err(ConnectionManagerError::ConnectionClosed)
        } else {
            Ok(())
        }
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        conn.is_closed()
    }
}
