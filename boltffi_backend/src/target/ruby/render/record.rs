//! Records as Ruby `Data` classes.
//!
//! A record crosses the boundary as a frozen `Data` instance with one member
//! per field. The extension builds each instance eagerly, so Ruby code reads
//! fields through plain `Data` readers that YJIT compiles like `attr_reader`.
//!
//! The extension allocates the instance, sets each member, and freezes it.
//! `Data.new` would build a keyword `Hash` and call `initialize` instead,
//! which costs more than the native call itself. A value that Rust returns
//! is complete, so it skips `initialize`, as `Marshal.load` does.
//!
//! A direct record also crosses as its C struct: the box and unbox functions
//! convert between that struct and the `Data` instance.

use std::collections::BTreeSet;

use askama::Template;
use boltffi_binding::{DirectRecordDecl, EncodedRecordDecl, Native, RecordDecl};

use crate::{
    bridge::c::CBridgeContract,
    core::{AuxChunk, Diagnostic, Emitted, Error, RenderContext, Result},
    target::ruby::{
        codec::{read::Reader, write::Writer},
        name_style::{Name, member},
        support::{Support, unsupported},
        symbol::{PrimitiveSymbols, RecordSymbols},
        syntax::{Constant, Identifier},
    },
};

use super::function::direct_record_type;

#[derive(Template)]
#[template(path = "target/ruby/record.c", escape = "none")]
struct RecordTemplate<'record> {
    record: &'record Record,
}

#[derive(Template)]
#[template(path = "target/ruby/record_forward.c", escape = "none")]
struct ForwardTemplate<'record> {
    record: &'record Record,
}

/// One rendered record.
pub struct Record {
    symbols: RecordSymbols,
    members: Vec<Identifier>,
    body: Body,
    unbound_methods: bool,
}

enum Body {
    Direct(DirectBody),
    Encoded(EncodedBody),
}

struct DirectBody {
    c_type: String,
    fields: Vec<DirectField>,
}

struct EncodedBody {
    fields: Vec<EncodedField>,
    helpers: Vec<String>,
}

struct DirectField {
    c_name: String,
    stem: &'static str,
}

struct EncodedField {
    read: String,
    write: String,
}

/// The `Data` class definition for one rendered record.
pub struct Registration {
    pub constant: Constant,
    pub class: String,
    pub members: Vec<Identifier>,
}

impl Record {
    pub fn from_declaration(
        declaration: &RecordDecl<Native>,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        Support::new(context).record(declaration.id())?;
        let symbols = RecordSymbols::for_record(declaration)?;
        let body = match declaration {
            RecordDecl::Direct(record) => Self::direct(record, bridge)?,
            RecordDecl::Encoded(record) => Self::encoded(record, &symbols, context)?,
            _ => return unsupported("unknown record declaration"),
        };
        Ok(Self {
            symbols,
            members: Registration::from_declaration(declaration)?.members,
            body,
            unbound_methods: !declaration.methods().is_empty()
                || !declaration.initializers().is_empty(),
        })
    }

    pub fn render(&self) -> Result<Emitted> {
        let emitted = Emitted::primary(RecordTemplate { record: self }.render()?).with_aux(
            AuxChunk::ForwardDecl(ForwardTemplate { record: self }.render()?.into()),
        );
        if self.unbound_methods {
            return Ok(emitted.with_diagnostics([Diagnostic::new(
                "record methods and initializers are not supported by the Ruby target",
            )]));
        }
        Ok(emitted)
    }

    fn direct(record: &DirectRecordDecl<Native>, bridge: &CBridgeContract) -> Result<Body> {
        let c_record =
            bridge
                .source_direct_record(record.id())
                .ok_or(Error::BrokenBridgeContract {
                    bridge: "c",
                    invariant: "direct record has no C typedef",
                })?;
        let fields = record
            .fields()
            .iter()
            .zip(c_record.fields())
            .map(|(field, c_field)| {
                Ok(DirectField {
                    c_name: c_field.name().to_owned(),
                    stem: PrimitiveSymbols::new(field.ty().primitive()).stem()?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Body::Direct(DirectBody {
            c_type: direct_record_type(record.id(), bridge)?,
            fields,
        }))
    }

    fn encoded(
        record: &EncodedRecordDecl<Native>,
        symbols: &RecordSymbols,
        context: &RenderContext<Native>,
    ) -> Result<Body> {
        let mut reader = Reader::new(context);
        let mut helpers = Vec::new();
        let fields = record
            .fields()
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let read = reader.decode(field.read(), &format!("boltffi_fields[{index}]"))?;
                let mut writer = Writer::new(
                    context,
                    format!("boltffi_field_{index}"),
                    format!("{}_field{index}", symbols.helper_prefix()),
                )
                .root_path(vec![field.key().clone()]);
                let write = writer.encode(field.write())?;
                helpers.extend(writer.into_helpers());
                Ok(EncodedField { read, write })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Body::Encoded(EncodedBody { fields, helpers }))
    }

    /// Size of the C array that holds member values; C arrays cannot be empty.
    fn value_slots(&self) -> usize {
        self.members.len().max(1)
    }
}

impl Registration {
    pub fn from_declaration(declaration: &RecordDecl<Native>) -> Result<Self> {
        let members = match declaration {
            RecordDecl::Direct(record) => record
                .fields()
                .iter()
                .map(|field| member(field.key()))
                .collect::<Result<Vec<_>>>()?,
            RecordDecl::Encoded(record) => record
                .fields()
                .iter()
                .map(|field| member(field.key()))
                .collect::<Result<Vec<_>>>()?,
            _ => return unsupported("unknown record declaration"),
        };
        let constant = Name::new(declaration.name()).constant()?;
        // Escaping can turn two Rust fields into one member, such as `hash` and
        // `hash_`. `Data.define` raises for a duplicate member at load time.
        let mut seen = BTreeSet::new();
        if let Some(duplicate) = members.iter().find(|member| !seen.insert(member.as_str())) {
            return Err(Error::RubyNameCollision {
                scope: format!("record {constant} members"),
                name: duplicate.as_str().to_owned(),
            });
        }
        Ok(Self {
            constant,
            class: RecordSymbols::for_record(declaration)?.class(),
            members,
        })
    }
}
