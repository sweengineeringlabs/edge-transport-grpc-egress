//! [`ClosedPort`] -- a port on `127.0.0.1` that refuses every connection and cannot be given away.

use std::io;
use std::net::SocketAddr;

use socket2::{Domain, Protocol, Socket, Type};

/// A port that is bound but never listens, held for as long as this value lives.
///
/// A connection to a bound port that does not listen is refused. Holding the socket matters: a
/// port that was only free for an instant can be handed to another test's listener by the operating
/// system as soon as it is released, and a "closed" endpoint would then answer.
pub struct ClosedPort {
    port: u16,
    // Held only so that the operating system keeps the port reserved.
    _socket: Socket,
}

impl ClosedPort {
    /// Reserve a port the operating system picks on `127.0.0.1`.
    ///
    /// # Errors
    ///
    /// The error of creating or binding the socket, or of reading its address.
    pub fn reserve() -> io::Result<Self> {
        let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
        let any: SocketAddr = ([127, 0, 0, 1], 0).into();
        socket.bind(&any.into())?;
        let port = socket
            .local_addr()?
            .as_socket()
            .ok_or_else(|| io::Error::other("the bound address is not an internet address"))?
            .port();
        Ok(Self {
            port,
            _socket: socket,
        })
    }

    /// The reserved port number.
    pub fn get_port(&self) -> u16 {
        self.port
    }

    /// An `http://127.0.0.1:<port>` endpoint for the reserved port.
    pub fn get_endpoint(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}
