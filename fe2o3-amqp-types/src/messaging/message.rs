use serde::{Deserialize, Serialize, de::{self, VariantAccess}};
use serde_amqp::{value::Value, format_code::EncodingCodes, descriptor::Descriptor, described::Described};

use super::{
    ApplicationProperties, Data, DeliveryAnnotations, Footer, Header, MessageAnnotations,
    Properties, AmqpSequence, AmqpValue,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub header: Header,
    pub delivery_annotations: DeliveryAnnotations,
    pub message_annotations: MessageAnnotations,
    pub properties: Properties,
    pub application_properties: ApplicationProperties,
    pub body_section: BodySection,
    pub footer: Footer,
}

#[derive(Debug, Clone)]
pub enum BodySection {
    Data(Vec<Data>),
    Sequence(Vec<AmqpSequence>),
    Value(AmqpValue)
}

impl Serialize for BodySection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer 
    {
        match self {
            BodySection::Data(data) => data.serialize(serializer),
            BodySection::Sequence(seq) => seq.serialize(serializer),
            BodySection::Value(val) => val.serialize(serializer)
        }
    }
}

struct FieldVisitor { }

#[derive(Debug)]
enum Field {
    DataOrSequence,
    Value,
}

impl<'de> de::Visitor<'de> for FieldVisitor {
    type Value = Field;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("BodySection variant. One of Vec<Data>, Vec<AmqpSequence>, AmqpValue")
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: de::Error, 
    {
        let val = match v.try_into()
            .map_err(|_| de::Error::custom("Invalid format code for message body"))? 
        {
            EncodingCodes::DescribedType => Field::Value,
            EncodingCodes::List0 
            | EncodingCodes::List8 
            | EncodingCodes::List32 => Field::DataOrSequence,
            _ => return Err(de::Error::custom("Invalid format code for message body"))
        };
        Ok(val)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error, 
    {
        match v {
            0x0000_0000_0000_0077 => Ok(Field::Value),
            _ => return Err(de::Error::custom("Invalid descriptor code"))
        }
    }
}

impl<'de> de::Deserialize<'de> for Field {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> 
    {
        deserializer.deserialize_ignored_any(FieldVisitor{})
    }
}

struct Visitor { }

impl<'de> de::Visitor<'de> for Visitor {
    type Value = BodySection;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("enum BodySection")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>, 
    {
        let (val, variant) = data.variant()?;

        if let Field::Value = val {
            let value = variant.newtype_variant()?;
            Ok(BodySection::Value(value))
        } else {
            let values: Vec<Described<Value>> = variant.newtype_variant()?;

            let descriptor = values.first()
                .ok_or_else(|| de::Error::custom("Expecting either Data or AmqpSequence"))?
                .descriptor
                .as_ref();

            match descriptor {
                Descriptor::Code(code) => {
                    match code {
                        // Data
                        0x0000_0000_0000_0075 => {
                            let data: Result<Vec<Data>, _> = values.into_iter()
                                .map(|d| Data::try_from(*d.value))
                                .collect();
                            data
                                .map(|v| BodySection::Data(v))
                                .map_err(|_| de::Error::custom("Expecting Data"))
                        },
                        // Value
                        0x0000_0000_0000_0076 => {
                            let seq: Result<Vec<AmqpSequence>, _> = values.into_iter()
                                .map(|d| AmqpSequence::try_from(*d.value))
                                .collect();
                            seq
                                .map(|v| BodySection::Sequence(v))
                                .map_err(|_| de::Error::custom("Expecting AmqpSequence"))
                        },
                        _ => return Err(de::Error::custom("Expecting either Data or AmqpSequence"))
                    }
                },
                Descriptor::Name(name) => {
                    match name.as_str() {
                        "amqp:data:binary" => {
                            let data: Result<Vec<Data>, _> = values.into_iter()
                                .map(|d| Data::try_from(*d.value))
                                .collect();
                            data
                                .map(|v| BodySection::Data(v))
                                .map_err(|_| de::Error::custom("Expecting Data"))
                        },
                        "amqp:amqp-sequence:list" => {
                            let seq: Result<Vec<AmqpSequence>, _> = values.into_iter()
                                .map(|d| AmqpSequence::try_from(*d.value))
                                .collect();
                            seq
                                .map(|v| BodySection::Sequence(v))
                                .map_err(|_| de::Error::custom("Expecting AmqpSequence"))
                        },
                        _ => return Err(de::Error::custom("Expecting either Data or AmqpSequence"))
                    }
                }
            }
        }
    }
}

impl<'de> de::Deserialize<'de> for BodySection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> 
    {
        deserializer.deserialize_enum(
            serde_amqp::__constants::UNTAGGED_ENUM, 
            &["Data", "Sequence", "Value"], 
            Visitor {}
        )
    }
}





#[cfg(test)]
mod tests {
    use std::vec;

    use serde_amqp::{to_vec, from_slice, value::Value};
    use serde_bytes::ByteBuf;

    use crate::messaging::{Data, message::BodySection, AmqpSequence};

    #[test]
    fn test_serialize_deserialize_body() {
        let data = b"amqp".to_vec();
        let data = vec![Data(ByteBuf::from(data))];
        let body = BodySection::Data(data);
        let serialized = to_vec(&body).unwrap();
        println!("{:x?}", serialized);
        let deserialized: BodySection = from_slice(&serialized).unwrap();
        println!("{:?}", deserialized);
    }

    #[test]
    fn test_field_deserializer() {
        // let data = b"amqp".to_vec();
        // let data = vec![Data(ByteBuf::from(data))];
        // let body = BodySection::Data(data);

        let body = BodySection::Sequence(
            vec![
                AmqpSequence(vec![Value::Bool(true)])
            ]
        );

        // let body = BodySection::Value(AmqpValue(Value::Bool(true)));

        let serialized = to_vec(&body).unwrap();
        println!("{:x?}", serialized);
        let field: BodySection = from_slice(&serialized).unwrap();
        println!("{:?}", field);
    }
}