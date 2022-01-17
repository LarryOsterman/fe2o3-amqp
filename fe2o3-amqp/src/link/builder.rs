use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

use fe2o3_amqp_types::{
    definitions::{Fields, Handle, ReceiverSettleMode, SenderSettleMode, SequenceNo},
    messaging::{Source, Target},
    primitives::{Symbol, ULong},
};
use tokio::sync::{mpsc, Notify, RwLock};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;

use crate::{
    connection::builder::DEFAULT_OUTGOING_BUFFER_SIZE,
    link::{Error, Link, LinkHandle, LinkIncomingItem},
    session::{self, SessionHandle},
    util::{Consumer, Producer},
};

use super::{
    role,
    state::{LinkFlowState, LinkFlowStateInner, LinkState, UnsettledMap},
    type_state::Attached,
    Receiver, Sender,
};

/// Type state for link::builder::Builder;
pub struct WithoutName;

/// Type state for link::builder::Builder;
pub struct WithName;

/// Type state for link::builder::Builder;
pub struct WithoutTarget;

/// Type state for link::builder::Builder;
pub struct WithTarget;

pub struct Builder<Role, NameState, Addr> {
    pub name: String,
    pub snd_settle_mode: SenderSettleMode,
    pub rcv_settle_mode: ReceiverSettleMode,
    pub source: Option<Source>,
    pub target: Option<Target>,

    /// This MUST NOT be null if role is sender,
    /// and it is ignored if the role is receiver.
    /// See subsection 2.6.7.
    pub initial_delivery_count: SequenceNo,

    pub max_message_size: Option<ULong>,
    pub offered_capabilities: Option<Vec<Symbol>>,
    pub desired_capabilities: Option<Vec<Symbol>>,
    pub properties: Option<Fields>,

    pub buffer_size: usize,

    // Type state markers
    role: PhantomData<Role>,
    name_state: PhantomData<NameState>,
    addr_state: PhantomData<Addr>,
}

impl<Role> Builder<Role, WithoutName, WithoutTarget> {
    pub(crate) fn new() -> Self {
        Self {
            name: Default::default(),
            snd_settle_mode: Default::default(),
            rcv_settle_mode: Default::default(),
            source: Default::default(),
            target: Default::default(),
            initial_delivery_count: Default::default(),
            max_message_size: Default::default(),
            offered_capabilities: Default::default(),
            desired_capabilities: Default::default(),
            properties: Default::default(),

            buffer_size: DEFAULT_OUTGOING_BUFFER_SIZE,
            role: PhantomData,
            name_state: PhantomData,
            addr_state: PhantomData,
        }
    }
}

impl<Role, Addr> Builder<Role, WithoutName, Addr> {
    pub fn name(self, name: impl Into<String>) -> Builder<Role, WithName, Addr> {
        Builder {
            name: name.into(),
            snd_settle_mode: self.snd_settle_mode,
            rcv_settle_mode: self.rcv_settle_mode,
            source: self.source,
            target: self.target,
            initial_delivery_count: self.initial_delivery_count,
            max_message_size: self.max_message_size,
            offered_capabilities: self.offered_capabilities,
            desired_capabilities: self.desired_capabilities,
            buffer_size: self.buffer_size,
            properties: Default::default(),

            role: self.role,
            name_state: PhantomData,
            addr_state: self.addr_state,
        }
    }
}

impl<Role, Addr> Builder<Role, WithName, Addr> {
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

impl<Role, NameState, Addr> Builder<Role, NameState, Addr> {
    pub fn sender(self) -> Builder<role::Sender, NameState, Addr> {
        Builder {
            name: self.name,
            snd_settle_mode: self.snd_settle_mode,
            rcv_settle_mode: self.rcv_settle_mode,
            source: self.source,
            target: self.target,
            initial_delivery_count: self.initial_delivery_count,
            max_message_size: self.max_message_size,
            offered_capabilities: self.offered_capabilities,
            desired_capabilities: self.desired_capabilities,
            buffer_size: self.buffer_size,
            properties: Default::default(),

            role: PhantomData,
            name_state: self.name_state,
            addr_state: self.addr_state,
        }
    }

    pub fn receiver(self) -> Builder<role::Receiver, NameState, Addr> {
        Builder {
            name: self.name,
            snd_settle_mode: self.snd_settle_mode,
            rcv_settle_mode: self.rcv_settle_mode,
            source: self.source,
            target: self.target,
            initial_delivery_count: self.initial_delivery_count,
            max_message_size: self.max_message_size,
            offered_capabilities: self.offered_capabilities,
            desired_capabilities: self.desired_capabilities,
            buffer_size: self.buffer_size,
            properties: Default::default(),

            role: PhantomData,
            name_state: self.name_state,
            addr_state: self.addr_state,
        }
    }

    pub fn sender_settle_mode(mut self, mode: SenderSettleMode) -> Self {
        self.snd_settle_mode = mode;
        self
    }

    pub fn receiver_settle_mode(mut self, mode: ReceiverSettleMode) -> Self {
        self.rcv_settle_mode = mode;
        self
    }

    pub fn source(mut self, source: impl Into<Source>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn target(self, target: impl Into<Target>) -> Builder<Role, NameState, WithTarget> {
        Builder {
            name: self.name,
            snd_settle_mode: self.snd_settle_mode,
            rcv_settle_mode: self.rcv_settle_mode,
            source: self.source,
            target: Some(target.into()), // setting target
            initial_delivery_count: self.initial_delivery_count,
            max_message_size: self.max_message_size,
            offered_capabilities: self.offered_capabilities,
            desired_capabilities: self.desired_capabilities,
            buffer_size: self.buffer_size,
            properties: Default::default(),

            role: self.role,
            name_state: self.name_state,
            addr_state: PhantomData,
        }
    }

    pub fn max_message_size(mut self, max_size: impl Into<ULong>) -> Self {
        self.max_message_size = Some(max_size.into());
        self
    }

    pub fn add_offered_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.offered_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.offered_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    pub fn set_offered_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.offered_capabilities = Some(capabilities);
        self
    }

    pub fn add_desired_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.desired_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.desired_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    pub fn set_desired_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.desired_capabilities = Some(capabilities);
        self
    }

    pub fn properties(mut self, properties: Fields) -> Self {
        self.properties = Some(properties);
        self
    }

    async fn create_link_instance<C, M>(
        self,
        unsettled: Arc<RwLock<UnsettledMap<M>>>,
        output_handle: Handle,
        flow_state_consumer: C,
    ) -> Result<Link<Role, C, M>, Error> {
        let local_state = LinkState::Unattached;

        let max_message_size = match self.max_message_size {
            Some(s) => s,
            None => 0,
        };

        // Create a SenderLink instance
        let link = Link::<Role, C, M> {
            role: PhantomData,
            local_state,
            name: self.name,
            output_handle: Some(output_handle.clone()),
            input_handle: None,
            snd_settle_mode: self.snd_settle_mode,
            rcv_settle_mode: self.rcv_settle_mode,
            source: self.source, // TODO: how should this field be set?
            target: self.target,
            max_message_size,
            offered_capabilities: self.offered_capabilities,
            desired_capabilities: self.desired_capabilities,

            // delivery_count: self.initial_delivery_count,
            // properties: self.properties,
            // flow_state: Consumer::new(notifier, flow_state),
            flow_state: flow_state_consumer,
            unsettled,
        };
        Ok(link)
    }
}

impl<NameState, Addr> Builder<role::Sender, NameState, Addr> {
    /// This MUST NOT be null if role is sender,
    /// and it is ignored if the role is receiver.
    /// See subsection 2.6.7.
    pub fn initial_delivery_count(mut self, count: SequenceNo) -> Self {
        self.initial_delivery_count = count;
        self
    }
}

impl<NameState, Addr> Builder<role::Receiver, NameState, Addr> {}

impl Builder<role::Sender, WithName, WithTarget> {
    pub async fn attach(mut self, session: &mut SessionHandle) -> Result<Sender<Attached>, Error> {
        let buffer_size = self.buffer_size.clone();
        let (incoming_tx, incoming_rx) = mpsc::channel::<LinkIncomingItem>(self.buffer_size);
        let outgoing = PollSender::new(session.outgoing.clone());

        // Create shared link flow state
        let flow_state_inner = LinkFlowStateInner {
            initial_delivery_count: self.initial_delivery_count,
            delivery_count: self.initial_delivery_count,
            link_credit: 0, // The link-credit and available variables are initialized to zero.
            available: 0,
            drain: false, // The drain flag is initialized to false.
            properties: self.properties.take(),
        };
        let flow_state = Arc::new(LinkFlowState::sender(flow_state_inner));

        let unsettled = Arc::new(RwLock::new(BTreeMap::new()));
        let notifier = Arc::new(Notify::new());
        let flow_state_producer = Producer::new(notifier.clone(), flow_state.clone());
        let flow_state_consumer = Consumer::new(notifier, flow_state);
        let link_handle = LinkHandle::Sender {
            tx: incoming_tx,
            flow_state: flow_state_producer,
            unsettled: unsettled.clone(),
            receiver_settle_mode: Default::default(), // Update this on incoming attach
        };

        // Create Link in Session
        let output_handle =
            session::allocate_link(&mut session.control, self.name.clone(), link_handle).await?;

        let mut link = self
            .create_link_instance(unsettled, output_handle, flow_state_consumer)
            .await?;

        // Get writer to session
        let writer = session.outgoing.clone();
        let mut writer = PollSender::new(writer);
        let mut reader = ReceiverStream::new(incoming_rx);
        // Send an Attach frame
        super::do_attach(&mut link, &mut writer, &mut reader).await?;

        // Attach completed, return Sender
        let sender = Sender::<Attached> {
            link,
            buffer_size,
            session: session.control.clone(),
            outgoing,
            incoming: reader,
            marker: PhantomData,
        };
        Ok(sender)
    }
}

impl Builder<role::Receiver, WithName, WithTarget> {
    pub async fn attach(
        mut self,
        session: &mut SessionHandle,
    ) -> Result<Receiver<Attached>, Error> {
        let buffer_size = self.buffer_size.clone();
        let (incoming_tx, incoming_rx) = mpsc::channel::<LinkIncomingItem>(self.buffer_size);
        let outgoing = PollSender::new(session.outgoing.clone());

        // Create shared link flow state
        let flow_state_inner = LinkFlowStateInner {
            initial_delivery_count: self.initial_delivery_count,
            delivery_count: self.initial_delivery_count,
            link_credit: 0, // The link-credit and available variables are initialized to zero.
            available: 0,
            drain: false, // The drain flag is initialized to false.
            properties: self.properties.take(),
        };
        let flow_state = Arc::new(LinkFlowState::receiver(flow_state_inner));

        let unsettled = Arc::new(RwLock::new(BTreeMap::new()));
        let flow_state_producer = flow_state.clone();
        let flow_state_consumer = flow_state;
        let link_handle = LinkHandle::Receiver {
            tx: incoming_tx,
            flow_state: flow_state_producer,
            unsettled: unsettled.clone(),
            receiver_settle_mode: Default::default(), // Update this on incoming attach
            more: false,
        };

        // Create Link in Session
        let output_handle =
            session::allocate_link(&mut session.control, self.name.clone(), link_handle).await?;

        let mut link = self
            .create_link_instance(unsettled, output_handle, flow_state_consumer)
            .await?;

        // Get writer to session
        let writer = session.outgoing.clone();
        let mut writer = PollSender::new(writer);
        let mut reader = ReceiverStream::new(incoming_rx);
        // Send an Attach frame
        super::do_attach(&mut link, &mut writer, &mut reader).await?;

        let receiver = Receiver::<Attached> {
            link,
            buffer_size,
            session: session.control.clone(),
            outgoing,
            incoming: reader,
            marker: PhantomData,
            incomplete_transfer: None,
        };
        Ok(receiver)
    }
}
