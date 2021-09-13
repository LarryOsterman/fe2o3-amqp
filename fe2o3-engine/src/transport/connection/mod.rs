use std::{collections::{BTreeMap, HashMap}, convert::TryInto, marker::{self, PhantomData}, sync::Arc};

use crate::error::EngineError;
pub use crate::transport::Transport;
use fe2o3_types::{definitions::Milliseconds, performatives::{ChannelMax, MaxFrameSize, Open}};
use tokio::{net::TcpStream, sync::mpsc::{Sender, Receiver}};
use url::Url;

use self::{builder::WithoutContainerId, mux::MuxHandle};

use super::{amqp::{Frame, FrameBody}, protocol_header::ProtocolHeader, session::SessionHandle};

mod builder;
pub use builder::{Builder};
pub mod mux;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct InChanId(pub u16);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutChanId(pub u16);

impl From<u16> for OutChanId {
    fn from(val: u16) -> Self {
        Self(val)
    }
}

#[derive(Debug, Clone,)]
pub enum ConnectionState {
    Start,
    
    HeaderReceived,

    HeaderSent,

    HeaderExchange,

    OpenPipe,

    OpenClosePipe,

    OpenReceived,

    OpenSent,

    ClosePipe,

    Opened,

    CloseReceived,
    
    CloseSent,

    Discarding,

    End,
}

// brw can still be used as background task
// The broker message can be 
// ```rust 
// enum Message {
    // Incoming(Frame),
    // Outgoing(Frame)
// }
// ```
pub struct Connection {
    // FIXME: is this really needed?
    // local_open: Arc<Open>, // parameters should be set using the builder and not change before reconnect
    mux: MuxHandle,
}

impl Connection {
    pub async fn open(
        container_id: String,
        max_frame_size: impl Into<MaxFrameSize>,
        channel_max: impl Into<ChannelMax>,
        url: impl TryInto<Url, Error=url::ParseError>
    ) -> Result<Connection, EngineError> {
        Connection::builder()
            .container_id(container_id)
            .max_frame_size(max_frame_size)
            .channel_max(channel_max)
            .open(url).await
    }

    pub async fn close(&mut self) -> Result<(), EngineError> {
        self.mux.control_mut().send(mux::MuxControl::Close).await?;
        Ok(())
    }

    pub fn mux(&self) -> &MuxHandle {
        &self.mux
    }

    pub fn mux_mut(&mut self) -> &mut MuxHandle {
        &mut self.mux
    }

    pub fn builder() -> Builder<WithoutContainerId> {
        Builder::new()
    }
}

impl From<MuxHandle> for Connection {
    fn from(mux: MuxHandle) -> Self {
        Self { mux }
    }
}

#[cfg(test)]
mod tests {
    use fe2o3_amqp::to_vec;
    use fe2o3_types::performatives::Open;
    use tokio::io::AsyncWriteExt;

    use crate::transport::connection::Connection;

    #[tokio::test]
    async fn test_connection_codec() {
        use tokio_test::io::Builder;
        let mock = Builder::new()
            .write(b"AMQP")
            .write(&[0, 1, 0, 0])
            .read(b"AMQP")
            .read(&[0, 1, 0, 0])
            .write(&[0, 0, 0, 26])
            .write(&[02, 0, 0, 0])
            .write(&[
                0x00, 0x53, 0x10, 0xC0, 0x19, 0x05, 0xA1, 0x04, 0x31, 0x32, 
                0x33, 0x34, 0xA1, 0x09, 0x31, 0x32, 0x37, 0x2E, 0x30, 0x2E, 
                0x30, 0x2E, 0x31, 0x52, 0x64, 0x60, 0x00, 0x09, 0x52, 0x0A
            ])
            .build();

        let _connection = Connection::builder()
            .container_id("1234")
            .hostname("127.0.0.1")
            .max_frame_size(100)
            .channel_max(9)
            .idle_time_out(10u32)
            .with_stream(mock).await
            .unwrap();


        // let open = Open{
        //     container_id: "1234".into(),
        //     hostname: Some("127.0.0.1".into()), 
        //     max_frame_size: 100.into(),
        //     channel_max: 9.into(),
        //     idle_time_out: Some(10),
        //     outgoing_locales: None,
        //     incoming_locales: None,
        //     offered_capabilities: None,
        //     desired_capabilities: None,
        //     properties: None
        // };

        // let vec = to_vec(&open).unwrap();
        // println!("{:x?}", vec);
    }
}