//! Names of the C symbols that the generated extension defines.
//!
//! Every record gets one class variable, one reader, and one writer. Direct
//! records also get a box and an unbox function for their C struct. The
//! codec and the renderers derive these names from here, so a call site
//! always names the function the record renderer emits.

use boltffi_binding::{Native, Primitive, RecordDecl, RecordId};

use crate::{
    bridge::c::{Identifier, Type, TypeFragment},
    core::{Error, RenderContext, Result},
};

use crate::target::ruby::name_style::Name;

/// C symbols for one record.
pub struct RecordSymbols {
    class: Identifier,
    builder: Identifier,
    reader: Identifier,
    writer: Identifier,
    boxer: Identifier,
    unboxer: Identifier,
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
        let stem = Name::new(record.name()).constant()?;
        let symbol = |role: &str| Identifier::parse(format!("boltffi_ruby_{role}_{stem}"));
        Ok(Self {
            class: symbol("class")?,
            builder: symbol("build")?,
            reader: symbol("read")?,
            writer: symbol("write")?,
            boxer: symbol("box")?,
            unboxer: symbol("unbox")?,
        })
    }

    /// The static `VALUE` that holds the record's `Data` class.
    pub fn class(&self) -> &Identifier {
        &self.class
    }

    /// Builds the `Data` instance from member values in declaration order.
    pub fn builder(&self) -> &Identifier {
        &self.builder
    }

    /// Reads the record from an encoded buffer into a Ruby object.
    pub fn reader(&self) -> &Identifier {
        &self.reader
    }

    /// Writes a Ruby object into an encoded buffer.
    pub fn writer(&self) -> &Identifier {
        &self.writer
    }

    /// Converts the direct record's C struct into a Ruby object.
    pub fn boxer(&self) -> &Identifier {
        &self.boxer
    }

    /// Converts a Ruby object into the direct record's C struct.
    pub fn unboxer(&self) -> &Identifier {
        &self.unboxer
    }
}

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

    /// Reads one encoded value into a Ruby value.
    pub fn reader(&self) -> Result<Identifier> {
        self.helper("read")
    }

    /// Reads one element of a direct vector into a Ruby value.
    ///
    /// A direct vector holds `isize` and `usize` at native width, but the
    /// encoded readers always take 8 bytes for them.
    pub fn element_reader(&self) -> Result<Identifier> {
        match self.primitive {
            Primitive::ISize | Primitive::USize => self.helper("read_native"),
            _ => self.reader(),
        }
    }

    /// Checks one Ruby value and appends its encoded bytes.
    pub fn writer(&self) -> Result<Identifier> {
        self.helper("write")
    }

    /// The C ABI type of the primitive.
    pub fn c_type(&self) -> Result<TypeFragment> {
        TypeFragment::anonymous(&Type::primitive(self.primitive)?)
    }

    /// The number of bytes the primitive takes in an encoded buffer.
    pub fn wire_size(&self) -> usize {
        self.primitive.wire_size().get() as usize
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
