use std::io;

use crate::error::Error;

use super::{private, Read};

pub struct SliceReader<'s> {
    slice: &'s [u8]
}

impl<'s> SliceReader<'s> {
    pub fn new(slice: &'s [u8]) -> Self {
        Self {
            slice
        }
    }

    pub fn unexpected_eof(msg: &str) -> Error {
        Error::Io(io::Error::new(
            io::ErrorKind::UnexpectedEof, 
            msg
        ))
    }

    pub fn get_byte_slice(&mut self, n: usize) -> Result<&'s [u8], Error> {
        if self.slice.len() < n {
            return Err(Self::unexpected_eof(""))
        }
        let (read_slice, remaining) = self.slice.split_at(n);
        self.slice = remaining;
        Ok(read_slice)
    }
}

impl<'s> private::Sealed for SliceReader<'s> { }

impl<'s> Read<'s> for SliceReader<'s> {
    fn peek(&mut self) -> Result<u8, Error> {
        match self.slice.first() {
            Some(b) => Ok(*b),
            None => Err(Self::unexpected_eof("")),
        }
    }

    fn next(&mut self) -> Result<u8, Error> {
        match self.slice.len() {
            0 => Err(Self::unexpected_eof("")),
            _ => {
                // let (next, remaining) = self.slice.split_at(1);
                // self.slice = remaining;
                // Ok(next[0])
                let buf = self.get_byte_slice(1)?;
                Ok(buf[0])
            }
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        let n = buf.len();
        
        if self.slice.len() < n {
            Err(Self::unexpected_eof(""))
        } else {
            // let (read_slice, remaining) = self.slice.split_at(n);
            // self.slice = remaining;
            // Ok(())
            let read_slice = self.get_byte_slice(n)?;
            buf.copy_from_slice(read_slice);
            Ok(())
        }
    }

    fn forward_read_bytes<V>(&mut self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'s> 
    {
        visitor.visit_borrowed_bytes(self.get_byte_slice(len)?)
    }

    fn forward_read_str<V>(&mut self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'s> 
    {
        let str_slice = std::str::from_utf8(self.get_byte_slice(len)?)?;
        visitor.visit_borrowed_str(str_slice)
    }
}