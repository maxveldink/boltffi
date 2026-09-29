//! Names of the runtime helpers that the generated extension calls.

use boltffi_binding::Primitive;

use crate::{
    bridge::c::{Identifier, Type, TypeFragment},
    core::{Error, Result},
};

/// Runtime helpers and C type for one primitive.
pub struct PrimitiveSymbols {
    primitive: Primitive,
}

impl PrimitiveSymbols {
    pub fn new(primitive: Primitive) -> Self {
        Self { primitive }
    }

    /// Converts a Ruby value into the C value, such as `boltffi_ruby_to_i32`.
    pub fn ruby_to_c(&self) -> Result<Identifier> {
        self.helper("to")
    }

    /// Converts the C value into a Ruby value, such as `boltffi_ruby_from_i32`.
    pub fn c_to_ruby(&self) -> Result<Identifier> {
        self.helper("from")
    }

    /// The C ABI type of the primitive.
    pub fn c_type(&self) -> Result<TypeFragment> {
        TypeFragment::anonymous(&Type::primitive(self.primitive)?)
    }

    fn helper(&self, role: &str) -> Result<Identifier> {
        let stem = match self.primitive {
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
        };
        Identifier::parse(format!("boltffi_ruby_{role}_{stem}"))
    }
}
