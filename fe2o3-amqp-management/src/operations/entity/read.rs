use std::borrow::Cow;

use fe2o3_amqp_types::{
    messaging::{ApplicationProperties, Message},
    primitives::{OrderedMap, Value},
};

use crate::{
    constants::{IDENTITY, NAME, OPERATION, READ},
    error::{Error, Result},
    request::MessageSerializer,
    response::MessageDeserializer,
};

pub trait Read {
    fn read(&mut self, arg: ReadRequest) -> Result<ReadResponse>;
}

/// Retrieve the attributes of a Manageable Entity.
///
/// Exactly one of name or identity MUST be provided
///
/// Body: No information is carried in the message body therefore any message body is valid and MUST
/// be ignored
#[derive(Debug)]
pub enum ReadRequest<'a> {
    /// The name of the Manageable Entity to be managed. This is case-sensitive.
    Name(Cow<'a, str>),

    /// The identity of the Manageable Entity to be managed. This is case-sensitive.
    Identity(Cow<'a, str>),
}

impl<'a> ReadRequest<'a> {
    /// The name of the Manageable Entity to be managed. This is case-sensitive.
    pub fn name(value: impl Into<Cow<'a, str>>) -> Self {
        Self::Name(value.into())
    }

    /// The identity of the Manageable Entity to be managed. This is case-sensitive.
    pub fn identity(value: impl Into<Cow<'a, str>>) -> Self {
        Self::Identity(value.into())
    }
}

impl<'a> MessageSerializer for ReadRequest<'a> {
    type Body = ();

    fn into_message(self) -> Message<Self::Body> {
        let (key, value) = match self {
            ReadRequest::Name(value) => (NAME, value),
            ReadRequest::Identity(value) => (IDENTITY, value),
        };

        Message::builder()
            .application_properties(
                ApplicationProperties::builder()
                    .insert(OPERATION, READ)
                    .insert(key, &value[..])
                    .build(),
            )
            .body(())
            .build()
    }
}

#[derive(Debug)]
pub struct ReadResponse {
    pub entity_attributes: OrderedMap<String, Value>,
}

impl ReadResponse {
    pub const STATUS_CODE: u16 = 200;
}

impl MessageDeserializer<Option<OrderedMap<String, Value>>> for ReadResponse {
    type Error = Error;

    fn from_message(message: Message<Option<OrderedMap<String, Value>>>) -> Result<Self> {
        match message.body {
            Some(map) => Ok(Self { entity_attributes: map }),
            None => Ok(Self { entity_attributes: OrderedMap::with_capacity(0) }),
        }
    }
}
