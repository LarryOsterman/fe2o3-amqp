mod array;
mod decimal;
mod symbol;
mod timestamp;
mod uuid;

// to avoid ambiguity
pub use crate::primitives::array::*;
pub use crate::primitives::decimal::*;
pub use crate::primitives::symbol::*;
pub use crate::primitives::timestamp::*;
pub use crate::primitives::uuid::*;

// Alias for the primitive types to match those in the spec
use serde_bytes::ByteBuf;

pub type Boolean = bool;
pub type UByte = u8;
pub type UShort = u16;
pub type UInt = u32;
pub type ULong = u64;
pub type Byte = i8;
pub type Short = i16;
pub type Int = i32;
pub type Long = i64;
pub type Float = f32;
pub type Double = f64;
pub type Char = char;
pub type Binary = ByteBuf;