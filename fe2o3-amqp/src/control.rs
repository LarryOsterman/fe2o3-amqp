//! Controls for Connection, Session, and Link

use fe2o3_amqp_types::definitions::{self, Handle};
use tokio::sync::{mpsc::Sender, oneshot};

use crate::{
    connection::{engine::SessionId, AllocSessionError},
    endpoint::LinkFlow,
    link::LinkHandle,
    session::{AllocLinkError, SessionIncomingItem},
};

#[derive(Debug)]
pub enum ConnectionControl {
    Open,
    Close(Option<definitions::Error>),
    CreateSession {
        tx: Sender<SessionIncomingItem>,
        responder: oneshot::Sender<Result<(u16, SessionId), AllocSessionError>>,
    },
    DropSession(SessionId),
}

pub enum SessionControl {
    Begin,
    End(Option<definitions::Error>),
    CreateLink {
        link_handle: LinkHandle,
        responder: oneshot::Sender<Result<Handle, AllocLinkError>>,
    },
    DropLink(Handle),
    LinkFlow(LinkFlow),
}

pub enum LinkControl {}
