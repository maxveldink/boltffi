//! Names of the C symbols that the generated extension defines.
//!
//! Every record gets one class variable, one reader, and one writer. Direct
//! records also get a box and an unbox function for their C struct. The
//! codec and the renderers derive these names from here, so a call site
//! always names the function the record renderer emits.

use boltffi_binding::{Native, Primitive, RecordDecl, RecordId};

use crate::core::{Error, RenderContext, Result};

use super::name_style::Name;

/// C symbols for one record.
pub struct RecordSymbols {
    stem: String,
}

impl RecordSymbols {
    pub fn new(id: RecordId, context: &RenderContext<Native>) -> Result<Self> {
        let record = context.record(id).ok_or(Error::UnexpectedBindingShape {
            layer: "ruby host",
            shape: "missing record declaration",
        })?;
        Self::for_record(record)
    }

    pub fn for_record(record: &RecordDecl<Native>) -> Result<Self> {
        Ok(Self {
            stem: Name::new(record.name()).constant()?.to_string(),
        })
    }

    /// The static `VALUE` that holds the record's `Data` class.
    pub fn class(&self) -> String {
        format!("boltffi_ruby_class_{}", self.stem)
    }

    /// Builds the `Data` instance from member values in declaration order.
    pub fn builder(&self) -> String {
        format!("boltffi_ruby_build_{}", self.stem)
    }

    /// Reads the record from an encoded buffer into a Ruby object.
    pub fn reader(&self) -> String {
        format!("boltffi_ruby_read_{}", self.stem)
    }

    /// Writes a Ruby object into an encoded buffer.
    pub fn writer(&self) -> String {
        format!("boltffi_ruby_write_{}", self.stem)
    }

    /// Converts the direct record's C struct into a Ruby object.
    pub fn boxer(&self) -> String {
        format!("boltffi_ruby_box_{}", self.stem)
    }

    /// Converts a Ruby object into the direct record's C struct.
    pub fn unboxer(&self) -> String {
        format!("boltffi_ruby_unbox_{}", self.stem)
    }

    /// Prefix for helpers the record writer needs, such as map callbacks.
    pub fn helper_prefix(&self) -> String {
        format!("boltffi_ruby_write_{}", self.stem)
    }
}

/// Runtime helper suffix and C type for one primitive.
pub struct PrimitiveSymbols {
    primitive: Primitive,
}

impl PrimitiveSymbols {
    pub fn new(primitive: Primitive) -> Self {
        Self { primitive }
    }

    /// Suffix of the `boltffi_ruby_to_*`, `from_*`, `read_*`, and `write_*` helpers.
    pub fn stem(&self) -> Result<&'static str> {
        Ok(match self.primitive {
            Primitive::Bool => "bool",
            Primitive::I8 => "i8",
            Primitive::U8 => "u8",
            Primitive::I16 => "i16",
            Primitive::U16 => "u16",
            Primitive::I32 => "i32",
            Primitive::U32 => "u32",
            Primitive::I64 => "i64",
            Primitive::U64 => "u64",
            Primitive::ISize => "isize",
            Primitive::USize => "usize",
            Primitive::F32 => "f32",
            Primitive::F64 => "f64",
            _ => {
                return Err(Error::UnsupportedTarget {
                    target: "ruby",
                    shape: "unknown primitive",
                });
            }
        })
    }

    /// The C ABI type of the primitive.
    pub fn c_type(&self) -> Result<&'static str> {
        Ok(match self.primitive {
            Primitive::Bool => "bool",
            Primitive::I8 => "int8_t",
            Primitive::U8 => "uint8_t",
            Primitive::I16 => "int16_t",
            Primitive::U16 => "uint16_t",
            Primitive::I32 => "int32_t",
            Primitive::U32 => "uint32_t",
            Primitive::I64 => "int64_t",
            Primitive::U64 => "uint64_t",
            Primitive::ISize => "intptr_t",
            Primitive::USize => "uintptr_t",
            Primitive::F32 => "float",
            Primitive::F64 => "double",
            _ => {
                return Err(Error::UnsupportedTarget {
                    target: "ruby",
                    shape: "unknown primitive",
                });
            }
        })
    }

    /// The number of bytes the primitive takes in an encoded buffer.
    pub fn wire_size(&self) -> usize {
        self.primitive.wire_size().get() as usize
    }
}
