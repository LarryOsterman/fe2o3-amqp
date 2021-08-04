use std::{cell::RefCell, collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque}, rc::{Rc, Weak}, sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard}};

use ordered_float::OrderedFloat;
use serde_bytes::ByteBuf;
use uuid::Uuid;

use crate::types::{Dec128, Dec32, Dec64, Symbol, Timestamp};

#[derive(Debug, Clone)]
pub enum EncodingType {
    Seq,
    Map
}

pub struct Contract {
    pub name: Option<String>,
    pub code: Option<u64>,
    pub encoding_type: Option<EncodingType>
}

impl Default for Contract {
    fn default() -> Self {
        Self { 
            name: None, 
            code: None, 
            encoding_type: None 
        }
    }
}

impl Contract {
    pub fn from_type<T: AmqpContract>() -> Self {
        Self {
            name: T::get_name(),
            code: T::get_code(),
            encoding_type: T::get_encoding_type()
        }
    }

    pub fn get_name(&self) -> &Option<String> { 
        &self.name
    }

    pub fn get_code(&self) -> &Option<u64> {
        &self.code
    }

    pub fn get_encoding_type(&self) -> &Option<EncodingType> {
        &self.encoding_type
    }
}

pub trait AmqpContract {
    fn get_name() -> Option<String> { None }

    fn get_code() -> Option<u64> { None }

    fn get_encoding_type() -> Option<EncodingType> { None }
}

macro_rules! impl_amqp_contract_for_primitive_types {
    ($($primitive: ty),*) => {
        $(
            impl AmqpContract for $primitive { }
        )*
    };
}

impl_amqp_contract_for_primitive_types!(
    (), bool, u8, u16, u32, u64, u128, 
    i8, i16, i32, i64, f32, f64, OrderedFloat<f32>, OrderedFloat<f64>,
    Dec32, Dec64, Dec128, char, Timestamp, Uuid, 
    ByteBuf, &str, &mut str, String, Symbol
);

impl<T> AmqpContract for [T] { }

impl<T> AmqpContract for Vec<T> { }

impl<T> AmqpContract for VecDeque<T> { }

impl<T> AmqpContract for LinkedList<T> { }

impl<T> AmqpContract for HashSet<T> { }

impl<T> AmqpContract for BTreeSet<T> { }

impl<T> AmqpContract for BinaryHeap<T> { }

impl<T> AmqpContract for Option<T> { }

impl<O, E> AmqpContract for Result<O, E> { }

impl<K, V> AmqpContract for HashMap<K, V> { }

impl<K, V> AmqpContract for BTreeMap<K, V> { }

impl<T: AmqpContract> AmqpContract for &T { }

impl<T: AmqpContract> AmqpContract for &mut T { }

impl<T: AmqpContract> AmqpContract for Weak<T> { }

impl<T: AmqpContract> AmqpContract for Arc<T> { }

impl<T: AmqpContract> AmqpContract for Box<T> { }

impl<T: AmqpContract> AmqpContract for Rc<T> { }

impl<T: AmqpContract> AmqpContract for RefCell<T> { }

impl<T: AmqpContract> AmqpContract for Mutex<T> { }

impl<'a, T: AmqpContract> AmqpContract for MutexGuard<'a, T> { }

impl<T: AmqpContract> AmqpContract for RwLock<T> { }

impl<'a, T: AmqpContract> AmqpContract for RwLockReadGuard<'a, T> { }

impl<'a, T: AmqpContract> AmqpContract for RwLockWriteGuard<'a, T> { }