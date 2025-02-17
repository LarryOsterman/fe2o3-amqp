//! Value serializer

use std::convert::TryFrom;

use ordered_float::OrderedFloat;
use serde::ser::{self};
use serde_bytes::ByteBuf;

use crate::{
    __constants::{
        ARRAY, DECIMAL128, DECIMAL32, DECIMAL64, DESCRIBED_BASIC, DESCRIBED_LIST, DESCRIBED_MAP,
        DESCRIPTOR, LAZY_VALUE, SYMBOL, SYMBOL_REF, TIMESTAMP, UUID,
    },
    described::Described,
    descriptor::Descriptor,
    error::Error,
    primitives::{Array, Dec128, Dec32, Dec64, OrderedMap, Symbol, Timestamp, Uuid},
    read::SliceReader,
    util::{FieldRole, NonNativeType, SequenceType},
};

use super::Value;

impl ser::Serialize for Value {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Value::Described(v) => v.serialize(serializer),
            Value::Null => serializer.serialize_unit(),
            Value::Bool(v) => serializer.serialize_bool(*v),
            Value::Ubyte(v) => serializer.serialize_u8(*v),
            Value::Ushort(v) => serializer.serialize_u16(*v),
            Value::Uint(v) => serializer.serialize_u32(*v),
            Value::Ulong(v) => serializer.serialize_u64(*v),
            Value::Byte(v) => serializer.serialize_i8(*v),
            Value::Short(v) => serializer.serialize_i16(*v),
            Value::Int(v) => serializer.serialize_i32(*v),
            Value::Long(v) => serializer.serialize_i64(*v),
            Value::Float(v) => serializer.serialize_f32(v.into_inner()),
            Value::Double(v) => serializer.serialize_f64(v.into_inner()),
            Value::Decimal32(v) => v.serialize(serializer),
            Value::Decimal64(v) => v.serialize(serializer),
            Value::Decimal128(v) => v.serialize(serializer),
            Value::Char(v) => serializer.serialize_char(*v),
            Value::Timestamp(v) => v.serialize(serializer),
            Value::Uuid(v) => v.serialize(serializer),
            Value::Binary(v) => serializer.serialize_bytes(v.as_slice()),
            Value::String(v) => serializer.serialize_str(v),
            Value::Symbol(v) => v.serialize(serializer),
            Value::List(v) => v.serialize(serializer),
            Value::Map(v) => v.serialize(serializer),
            Value::Array(v) => v.serialize(serializer),
        }
    }
}

/// Serializes a instance of type `T` as an AMQP1.0 [`Value`]
pub fn to_value<T>(val: &T) -> Result<Value, Error>
where
    T: ser::Serialize,
{
    let mut ser = Serializer::new();
    ser::Serialize::serialize(val, &mut ser)
}

/// A structure that serializes types into [`Value`]
#[derive(Debug)]
pub struct Serializer {
    non_native_type: Option<NonNativeType>,
    seq_type: Option<SequenceType>,
}

impl Serializer {
    /// Creates a new value serializer
    pub fn new() -> Self {
        Self {
            non_native_type: None,
            seq_type: None,
        }
    }
}

impl Default for Serializer {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = SeqSerializer<'a>;
    type SerializeTuple = SeqSerializer<'a>;
    type SerializeTupleStruct = TupleStructSerializer<'a>;
    type SerializeStruct = StructSerializer<'a>;
    type SerializeMap = MapSerializer<'a>;
    type SerializeStructVariant = VariantSerializer<'a>;
    type SerializeTupleVariant = VariantSerializer<'a>;

    #[inline]
    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Bool(v))
    }

    #[inline]
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Byte(v))
    }

    #[inline]
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Short(v))
    }

    #[inline]
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Int(v))
    }

    #[inline]
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        match self.non_native_type {
            None => Ok(Value::Long(v)),
            Some(NonNativeType::Timestamp) => {
                self.non_native_type = None;
                Ok(Value::Timestamp(Timestamp::from(v)))
            }
            _ => Err(Error::InvalidValue),
        }
    }

    #[inline]
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Ubyte(v))
    }

    #[inline]
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Ushort(v))
    }

    #[inline]
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Uint(v))
    }

    #[inline]
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Ulong(v))
    }

    #[inline]
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Float(OrderedFloat::from(v)))
    }

    #[inline]
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Double(OrderedFloat::from(v)))
    }

    #[inline]
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Char(v))
    }

    #[inline]
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        match self.non_native_type {
            None => Ok(Value::String(String::from(v))),
            Some(NonNativeType::Symbol) | Some(NonNativeType::SymbolRef) => {
                self.non_native_type = None;
                Ok(Value::Symbol(Symbol::from(v)))
            }
            _ => Err(Error::InvalidValue),
        }
    }

    #[inline]
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        match self.non_native_type {
            None => Ok(Value::Binary(ByteBuf::from(v.to_vec()))),
            Some(NonNativeType::Dec32) => {
                self.non_native_type = None;
                Ok(Value::Decimal32(Dec32::try_from(v)?))
            }
            Some(NonNativeType::Dec64) => {
                self.non_native_type = None;
                Ok(Value::Decimal64(Dec64::try_from(v)?))
            }
            Some(NonNativeType::Dec128) => {
                self.non_native_type = None;
                Ok(Value::Decimal128(Dec128::try_from(v)?))
            }
            Some(NonNativeType::Uuid) => {
                self.non_native_type = None;
                Ok(Value::Uuid(Uuid::try_from(v)?))
            }
            Some(NonNativeType::LazyValue) => {
                use serde::Deserialize;

                // LazyValue is just the serialized bytes, so we need to deserialize it into a Value
                let reader = SliceReader::new(v);
                let mut de = crate::de::Deserializer::new(reader);
                let value = Value::deserialize(&mut de)?;
                Ok(value)
            }
            Some(NonNativeType::Timestamp) => Err(Error::InvalidValue),
            Some(NonNativeType::Symbol) => Err(Error::InvalidValue),
            Some(NonNativeType::SymbolRef) => Err(Error::InvalidValue),
        }
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Null)
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _ame: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_u32(variant_index)
    }

    #[inline]
    fn serialize_newtype_struct<T>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize + ?Sized,
    {
        if name == SYMBOL {
            self.non_native_type = Some(NonNativeType::Symbol);
        } else if name == SYMBOL_REF {
            self.non_native_type = Some(NonNativeType::SymbolRef);
        } else if name == ARRAY {
            self.seq_type = Some(SequenceType::Array);
        } else if name == DECIMAL32 {
            self.non_native_type = Some(NonNativeType::Dec32);
        } else if name == DECIMAL64 {
            self.non_native_type = Some(NonNativeType::Dec64);
        } else if name == DECIMAL128 {
            self.non_native_type = Some(NonNativeType::Dec128);
        } else if name == TIMESTAMP {
            self.non_native_type = Some(NonNativeType::Timestamp);
        } else if name == UUID {
            self.non_native_type = Some(NonNativeType::Uuid);
        } else if name == LAZY_VALUE {
            self.non_native_type = Some(NonNativeType::LazyValue);
        }
        value.serialize(self)
    }

    #[inline]
    fn serialize_newtype_variant<T>(
        self,
        name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize + ?Sized,
    {
        use crate::__constants::VALUE;

        if name == DESCRIPTOR || name == VALUE
        // || name == AMQP_ERROR || name == CONNECTION_ERROR || name == SESSION_ERROR || name == LINK_ERROR
        {
            value.serialize(self)
        } else {
            use ser::SerializeSeq;
            // FIXME: how should enum be represented in Value
            let mut state = self.serialize_seq(Some(2))?;
            state.serialize_element(&variant_index)?;
            state.serialize_element(value)?;
            state.end()
        }
    }

    #[inline]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize + ?Sized,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SeqSerializer::new(self, len.unwrap_or(0)))
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(SeqSerializer::new(self, len))
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        match name {
            DESCRIBED_LIST => Ok(TupleStructSerializer::list(self, len)),
            DESCRIBED_BASIC => Ok(TupleStructSerializer::basic(self)),
            _ => Ok(TupleStructSerializer::list_fields(self, len)),
        }
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(VariantSerializer::new(
            self,
            name,
            variant_index,
            variant,
            len,
        ))
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer {
            se: self,
            map: OrderedMap::new(),
        })
    }

    #[inline]
    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        match name {
            DESCRIBED_LIST => Ok(StructSerializer::list(self, len)),
            DESCRIBED_MAP => Ok(StructSerializer::map(self)),
            DESCRIBED_BASIC => Ok(StructSerializer::basic(self)),
            _ => Ok(StructSerializer::list(self, len)),
        }
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(VariantSerializer::new(
            self,
            name,
            variant_index,
            variant,
            len,
        ))
    }

    #[inline]
    fn is_human_readable(&self) -> bool {
        false
    }
}

/// Serializer for sequence types
#[derive(Debug)]
pub struct SeqSerializer<'a> {
    se: &'a mut Serializer,
    vec: Vec<Value>,
}

impl<'a> SeqSerializer<'a> {
    pub(crate) fn new(se: &'a mut Serializer, len: usize) -> Self {
        Self {
            se,
            vec: Vec::with_capacity(len),
        }
    }
}

impl AsMut<Serializer> for SeqSerializer<'_> {
    fn as_mut(&mut self) -> &mut Serializer {
        self.se
    }
}

impl ser::SerializeSeq for SeqSerializer<'_> {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize + ?Sized,
    {
        let mut se = Serializer::new();
        let val: Value = value.serialize(&mut se)?;
        self.vec.push(val);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        match self.se.seq_type {
            None | Some(SequenceType::List) => Ok(Value::List(self.vec)),
            Some(SequenceType::Array) => Ok(Value::Array(Array::from(self.vec))),
            _ => Err(Error::InvalidValue),
        }
    }
}

impl ser::SerializeTuple for SeqSerializer<'_> {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize + ?Sized,
    {
        let mut se = Serializer::new();
        let val = value.serialize(&mut se)?;
        self.vec.push(val);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::List(self.vec))
    }
}

/// Serializer for tuple struct fields
#[derive(Debug)]
pub enum TupleStructSerializerKind<'a> {
    /// A transparent serializer
    Basic {
        /// A mutable reference to the value serializer
        se: &'a mut Serializer,

        /// A temporary placeholder for the serialized value
        val: Option<Value>,
    },

    /// Encode fields as a list
    List(SeqSerializer<'a>),
}

/// Serilaizer for tuple struct
#[derive(Debug)]
pub struct TupleStructSerializer<'a> {
    field_role: FieldRole,
    descriptor: Option<Descriptor>,
    kind: TupleStructSerializerKind<'a>,
}

impl<'a> TupleStructSerializer<'a> {
    fn basic(se: &'a mut Serializer) -> Self {
        let kind = TupleStructSerializerKind::Basic { se, val: None };
        Self {
            field_role: FieldRole::Descriptor,
            kind,
            descriptor: None,
        }
    }

    fn list(se: &'a mut Serializer, len: usize) -> Self {
        let kind = TupleStructSerializerKind::List(SeqSerializer::new(se, len));
        Self {
            field_role: FieldRole::Descriptor,
            kind,
            descriptor: None,
        }
    }

    fn list_fields(se: &'a mut Serializer, len: usize) -> Self {
        let kind = TupleStructSerializerKind::List(SeqSerializer::new(se, len));
        Self {
            field_role: FieldRole::Fields,
            kind,
            descriptor: None,
        }
    }
}

impl AsMut<Serializer> for TupleStructSerializer<'_> {
    fn as_mut(&mut self) -> &mut Serializer {
        match &mut self.kind {
            TupleStructSerializerKind::Basic { se, .. } => se,
            TupleStructSerializerKind::List(seq) => seq.as_mut(),
        }
    }
}

impl ser::SerializeTupleStruct for TupleStructSerializer<'_> {
    type Ok = Value;

    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize + ?Sized,
    {
        use ser::SerializeSeq;

        match self.field_role {
            FieldRole::Descriptor => {
                self.field_role = FieldRole::Fields;
                let value = value.serialize(self.as_mut())?;
                match value {
                    Value::Symbol(name) => self.descriptor = Some(Descriptor::Name(name)),
                    Value::Ulong(code) => self.descriptor = Some(Descriptor::Code(code)),
                    _ => return Err(Error::InvalidValue),
                }
                Ok(())
            }
            FieldRole::Fields => match &mut self.kind {
                TupleStructSerializerKind::Basic { val, .. } => {
                    let mut se = Serializer::new();
                    *val = Some(value.serialize(&mut se)?);
                    Ok(())
                }
                TupleStructSerializerKind::List(seq) => seq.serialize_element(value),
            },
        }
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        use serde::ser::SerializeSeq;
        let value = match self.kind {
            TupleStructSerializerKind::Basic { se: _, val } => val.ok_or(Error::InvalidValue)?,
            TupleStructSerializerKind::List(seq) => seq.end()?,
        };

        match self.descriptor {
            Some(descriptor) => {
                let described = Described { descriptor, value };
                Ok(Value::Described(Box::new(described)))
            }
            None => Ok(value),
        }
    }
}

/// Serializer for struct fields
#[derive(Debug)]
pub enum StructSerializerKind<'a> {
    /// A transparent serializer
    Basic {
        /// A mutable reference to the value serializer
        se: &'a mut Serializer,

        /// A temporary placeholder for the serialized value
        val: Option<Value>,
    },

    /// Encode fields as a list
    List(SeqSerializer<'a>),

    /// Encode fields as a map
    Map(MapSerializer<'a>),
}

/// Struct serializer
#[derive(Debug)]
pub struct StructSerializer<'a> {
    descriptor: Option<Descriptor>,
    kind: StructSerializerKind<'a>,
}

impl<'a> StructSerializer<'a> {
    /// Create a struct serializer with transparent encoding
    pub fn basic(se: &'a mut Serializer) -> Self {
        let kind = StructSerializerKind::Basic { se, val: None };
        Self {
            descriptor: None,
            kind,
        }
    }

    /// Create a struct serializer for list encoding
    pub fn list(se: &'a mut Serializer, len: usize) -> Self {
        let kind = StructSerializerKind::List(SeqSerializer::new(se, len));
        Self {
            descriptor: None,
            kind,
        }
    }

    /// Create a struct serializer for map encoding
    pub fn map(se: &'a mut Serializer) -> Self {
        let kind = StructSerializerKind::Map(MapSerializer::new(se));
        Self {
            descriptor: None,
            kind,
        }
    }
}

impl AsMut<Serializer> for StructSerializer<'_> {
    fn as_mut(&mut self) -> &mut Serializer {
        match &mut self.kind {
            StructSerializerKind::Basic { se, .. } => se,
            StructSerializerKind::List(seq) => seq.as_mut(),
            StructSerializerKind::Map(map) => map.as_mut(),
        }
    }
}

impl ser::SerializeStruct for StructSerializer<'_> {
    type Ok = Value;

    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize + ?Sized,
    {
        use serde::ser::{SerializeMap, SerializeSeq};

        if key == DESCRIPTOR {
            let value = value.serialize(self.as_mut())?;
            match value {
                Value::Symbol(name) => self.descriptor = Some(Descriptor::Name(name)),
                Value::Ulong(code) => self.descriptor = Some(Descriptor::Code(code)),
                _ => return Err(Error::InvalidValue),
            }
            Ok(())
        } else {
            match &mut self.kind {
                StructSerializerKind::Basic { val, .. } => {
                    let mut se = Serializer::new();
                    *val = Some(value.serialize(&mut se)?);
                    Ok(())
                }
                StructSerializerKind::List(seq) => seq.serialize_element(value),
                StructSerializerKind::Map(map) => map.serialize_entry(key, value),
            }
        }
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        use serde::ser::{SerializeMap, SerializeSeq};
        let value = match self.kind {
            StructSerializerKind::Basic { se: _, val } => val.ok_or(Error::InvalidValue)?,
            StructSerializerKind::List(seq) => seq.end()?,
            StructSerializerKind::Map(map) => map.end()?,
        };

        match self.descriptor {
            Some(descriptor) => {
                let described = Described { descriptor, value };
                Ok(Value::Described(Box::new(described)))
            }
            None => Ok(value),
        }
    }
}

/// Serializer for map types
#[derive(Debug)]
pub struct MapSerializer<'a> {
    se: &'a mut Serializer,
    map: OrderedMap<Value, Value>,
}

impl<'a> MapSerializer<'a> {
    /// Create a new map serializer
    pub fn new(se: &'a mut Serializer) -> Self {
        Self {
            se,
            map: Default::default(),
        }
    }
}

impl AsMut<Serializer> for MapSerializer<'_> {
    fn as_mut(&mut self) -> &mut Serializer {
        self.se
    }
}

impl ser::SerializeMap for MapSerializer<'_> {
    type Ok = Value;
    type Error = Error;

    fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<(), Self::Error>
    where
        K: serde::Serialize + ?Sized,
        V: serde::Serialize + ?Sized,
    {
        let mut se = Serializer::new();
        let key = key.serialize(&mut se)?;
        let value = value.serialize(&mut se)?;
        self.map.insert(key, value);
        Ok(())
    }

    fn serialize_key<T>(&mut self, _key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize + ?Sized,
    {
        unreachable!()
    }

    fn serialize_value<T>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize + ?Sized,
    {
        unreachable!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Map(self.map))
    }
}

/// Serializer for enum variants
#[derive(Debug)]
pub struct VariantSerializer<'a> {
    se: &'a mut Serializer,
    _name: &'static str,
    variant_index: u32,
    _variant: &'static str,
    _num: usize,
    buf: Vec<Value>,
}

impl<'a> VariantSerializer<'a> {
    pub(crate) fn new(
        se: &'a mut Serializer,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        num: usize, // number of field in the tuple
    ) -> Self {
        Self {
            se,
            _name: name,
            variant_index,
            _variant: variant,
            _num: num,
            buf: Vec::new(),
        }
    }
}

impl AsMut<Serializer> for VariantSerializer<'_> {
    fn as_mut(&mut self) -> &mut Serializer {
        self.se
    }
}

impl ser::SerializeTupleVariant for VariantSerializer<'_> {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize + ?Sized,
    {
        let mut se = Serializer::new();
        let value = value.serialize(&mut se)?;
        self.buf.push(value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let value = Value::List(self.buf);
        let index = Value::Uint(self.variant_index);
        Ok(Value::List(vec![index, value]))
    }
}

impl ser::SerializeStructVariant for VariantSerializer<'_> {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize + ?Sized,
    {
        <Self as ser::SerializeTupleVariant>::serialize_field(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        <Self as ser::SerializeTupleVariant>::end(self)
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    #[cfg(feature = "derive")]
    use serde_amqp_derive::{DeserializeComposite, SerializeComposite};

    #[cfg(feature = "derive")]
    use crate::{described::Described, from_slice, to_vec};

    use crate::{
        primitives::{Array, OrderedMap, Timestamp},
        value::Value,
    };

    use super::to_value;

    fn assert_eq_on_value_vs_expected<T: Serialize>(val: T, expected: Value) {
        let value: Value = to_value(&val).unwrap();
        assert_eq!(value, expected)
    }

    #[test]
    fn test_serialize_value_bool() {
        let val = true;
        let expected: Value = Value::Bool(true);
        assert_eq_on_value_vs_expected(val, expected)
    }

    #[test]
    fn test_serialize_value_timestamp() {
        let val = Timestamp::from(131313);
        let expected = Value::Timestamp(Timestamp::from(131313));
        assert_eq_on_value_vs_expected(val, expected);
    }

    #[test]
    fn test_serialize_value_list() {
        let val = vec![1i32, 2, 3, 4];
        let expected = Value::List(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
            Value::Int(4),
        ]);
        assert_eq_on_value_vs_expected(val, expected);
    }

    #[test]
    fn test_serialize_value_array() {
        let val = Array::from(vec![1i32, 2, 3, 4]);
        let expected = Value::Array(Array::from(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
            Value::Int(4),
        ]));
        println!("{:?}", val);
        assert_eq_on_value_vs_expected(val, expected);
    }

    #[test]
    fn test_serialize_value_map() {
        let mut val = OrderedMap::new();
        val.insert("a", 1i32);
        val.insert("m", 2);
        val.insert("q", 3);
        val.insert("p", 4);

        let mut expected: OrderedMap<Value, Value> = OrderedMap::new();
        expected.insert("a".into(), 1i32.into());
        expected.insert("m".into(), 2.into());
        expected.insert("q".into(), 3.into());
        expected.insert("p".into(), 4.into());
        let expected = Value::Map(expected);
        assert_eq_on_value_vs_expected(val, expected);
    }

    #[test]
    fn test_serialize_value_unit_variant() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        enum Foo {
            A,
            B,
            C,
        }

        let val = Foo::B;
        let expected = Value::Uint(1);
        assert_eq_on_value_vs_expected(val, expected);
    }

    #[test]
    fn test_serialize_value_newtype_variant() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        enum Foo {
            A(String),
            B(u64),
        }

        let val = Foo::B(13);
        let expected = Value::List(vec![Value::Uint(1), Value::Ulong(13)]);
        assert_eq_on_value_vs_expected(val, expected);
    }

    #[test]
    fn test_serialize_value_tuple_variant() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        enum Foo {
            A(u32, bool),
            B(i32, String),
        }
        let val = Foo::B(13, "amqp".to_string());
        let expected = Value::List(vec![
            Value::Uint(1),
            Value::List(vec![Value::Int(13), Value::String(String::from("amqp"))]),
        ]);
        assert_eq_on_value_vs_expected(val, expected);
    }

    #[test]
    fn test_serialize_value_struct_variant() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        enum Foo {
            A { num: u32, is_a: bool },
            B { signed_num: i32, amqp: String },
        }

        let val = Foo::A {
            num: 13,
            is_a: true,
        };
        let expected = Value::List(vec![
            Value::Uint(0),
            Value::List(vec![Value::Uint(13), Value::Bool(true)]),
        ]);
        assert_eq_on_value_vs_expected(val, expected);
    }

    #[derive(Debug, Serialize)]
    pub struct NewType<T>(T);

    #[derive(Debug, Serialize)]
    pub struct AnotherNewType<T> {
        inner: T,
    }

    #[cfg(feature = "derive")]
    use crate as serde_amqp;

    #[cfg(feature = "derive")]
    #[derive(Debug, SerializeComposite, DeserializeComposite)]
    #[amqp_contract(
        name = "composite",
        code = "0x0000_0000:0x0000_0001",
        encoding = "list"
    )]
    pub struct EmptyComposite {}

    #[cfg(feature = "derive")]
    #[derive(Debug, SerializeComposite, DeserializeComposite)]
    #[amqp_contract(
        name = "composite",
        code = "0x0000_0000:0x0000_0001",
        encoding = "list"
    )]
    pub struct Composite {
        a: i32,
        b: String,
    }

    #[test]
    fn test_serialize_vec_of_tuple() {
        // let data = vec![(&AnotherNewType{ inner: NewType(1i32) }, &NewType(false), &NewType("amqp"))];
        let data = AnotherNewType {
            inner: NewType(3i32),
        };
        let data = (data,);

        let buf = to_value(&data).unwrap();
        println!("{:?}", buf);
    }

    #[cfg(feature = "derive")]
    #[test]
    fn test_serialize_empty_composite() {
        let comp = EmptyComposite {};
        let value = to_value(&comp).unwrap();
        assert_eq!(
            value,
            Value::Described(Box::new(Described {
                descriptor: serde_amqp::descriptor::Descriptor::Code(1),
                value: Value::List(vec![])
            }))
        )
    }

    #[cfg(feature = "derive")]
    #[test]
    fn test_serialize_composite() {
        let comp = Composite {
            a: 1,
            b: "hello".to_string(),
        };
        let value = to_value(&comp).unwrap();
        assert_eq!(
            value,
            Value::Described(Box::new(Described {
                descriptor: serde_amqp::descriptor::Descriptor::Code(1),
                value: Value::List(vec![Value::Int(1), Value::String(String::from("hello"))])
            }))
        )
    }

    #[cfg(feature = "derive")]
    #[test]
    fn test_deserialize_empty_composite() {
        let comp = EmptyComposite {};
        let buf = to_vec(&comp).unwrap();
        let value: Value = from_slice(&buf).unwrap();
        assert_eq!(
            value,
            Value::Described(Box::new(Described {
                descriptor: serde_amqp::descriptor::Descriptor::Code(1),
                value: Value::List(vec![])
            }))
        )
    }

    #[cfg(feature = "derive")]
    #[test]
    fn test_serialize_nested_composite_some() {
        use crate::primitives::Symbol;
        #[derive(Debug, Clone, Default, DeserializeComposite, SerializeComposite)]
        #[amqp_contract(
            name = "amqp:source:list",
            code = "0x0000_0000:0x0000_0028",
            encoding = "list",
            rename_all = "kebab-case"
        )]
        pub struct Source {
            pub address: Option<String>,
            pub capabilities: Option<Array<Symbol>>,
        }

        #[derive(Debug, Clone, Default, DeserializeComposite, SerializeComposite)]
        #[amqp_contract(
            name = "amqp:target:list",
            code = "0x0000_0000:0x0000_0029",
            encoding = "list",
            rename_all = "kebab-case"
        )]
        pub struct Target {
            pub address: Option<String>,
            pub capabilities: Option<Array<Symbol>>,
        }

        #[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
        #[amqp_contract(
            name = "amqp:attach:list",
            code = "0x0000_0000:0x0000_0012",
            encoding = "list",
            rename_all = "kebab-case"
        )]
        pub struct Attach {
            pub source: Option<Source>,
            pub target: Option<Target>,
        }

        let source = Source {
            capabilities: Some(Array(Vec::new())),
            ..Default::default()
        };
        let target = Target {
            address: Some(String::from("some random address")),
            capabilities: Some(Array(vec![Symbol::from("x-azure-relay")])),
        };

        let frame = Attach {
            source: Some(source),
            target: Some(target),
        };
        assert!(to_value(&frame).is_ok());
    }
}
