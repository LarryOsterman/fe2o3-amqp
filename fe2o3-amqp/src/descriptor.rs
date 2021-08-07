use crate::types::Symbol;

#[derive(Debug)]
pub struct Descriptor {
    name: Symbol,
    code: Option<u64>
}