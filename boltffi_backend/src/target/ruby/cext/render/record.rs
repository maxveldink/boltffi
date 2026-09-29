//! Records as Ruby `Data` classes.
//!
//! A record crosses the boundary as a frozen `Data` instance with one member
//! per field. The extension builds each instance eagerly, so Ruby code reads
//! fields through plain `Data` readers that YJIT compiles like `attr_reader`.
//!
//! The extension builds each instance through the class's public `new`, so
//! a returned record is exactly what `Point.new(x:, y:)` returns, including
//! any `initialize` that the program defines. Writing the members directly
//! would be faster. That change can come later, with its tradeoffs.
//!
//! Arguments rely on Ruby storing a `Data` instance as a `Struct`:
//! `RSTRUCT_GET` reads each member. Reading through the member methods
//! instead would run Ruby code while the extension encodes an argument.
//!
//! A direct record also crosses as its C struct: the box and unbox functions
//! convert between that struct and the `Data` instance.

use askama::Template;
use boltffi_binding::{DirectRecordDecl, EncodedRecordDecl, FieldKey, Native, RecordDecl};

use crate::{
    bridge::c::{CBridgeContract, Expression, Identifier, Statement, TypeFragment},
    core::{AuxChunk, Diagnostic, Emitted, Error, RenderContext, Result},
    target::ruby::{
        cext::{
            codec::{read::Reader, write::Writer},
            support::{Support, unsupported},
            symbol::{PrimitiveSymbols, RecordSymbols},
        },
        name_style::{Name, NameScope, member, spelling},
        syntax::{Constant, Identifier as RubyIdentifier},
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
    members: Vec<RubyIdentifier>,
    body: Body,
    unbound_methods: bool,
}

enum Body {
    Direct(DirectBody),
    Encoded(EncodedBody),
}

struct DirectBody {
    c_type: TypeFragment,
    fields: Vec<DirectField>,
}

struct EncodedBody {
    fields: Vec<EncodedField>,
}

struct DirectField {
    c_name: Identifier,
    ruby_to_c: Identifier,
    c_to_ruby: Identifier,
}

struct EncodedField {
    read: Statement,
    write: Statement,
}

/// The `Data` class definition for one rendered record.
pub struct Registration {
    pub constant: Constant,
    pub class: Identifier,
    pub members: Vec<RubyIdentifier>,
    /// The Rust record, for name collision diagnostics.
    pub subject: String,
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
            RecordDecl::Encoded(record) => Self::encoded(record, context)?,
            _ => return unsupported("unknown record declaration"),
        };
        Ok(Self {
            symbols,
            members: field_keys(declaration)?
                .into_iter()
                .map(member)
                .collect::<Result<Vec<_>>>()?,
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
                let symbols = PrimitiveSymbols::new(field.ty().primitive());
                Ok(DirectField {
                    c_name: Identifier::parse(c_field.name())?,
                    ruby_to_c: symbols.ruby_to_c()?,
                    c_to_ruby: symbols.c_to_ruby()?,
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
        context: &RenderContext<Native>,
    ) -> Result<Body> {
        let mut reader = Reader::new(context);
        let fields = record
            .fields()
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let read = reader.decode(field.read(), &format!("boltffi_fields[{index}]"))?;
                let write = Writer::new(context, Expression::new(format!("boltffi_field_{index}")))
                    .root_path(vec![field.key().clone()])
                    .encode(field.write())?;
                Ok(EncodedField { read, write })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Body::Encoded(EncodedBody { fields }))
    }

    /// Size of the C array that holds member values; C arrays cannot be empty.
    fn value_slots(&self) -> usize {
        self.members.len().max(1)
    }
}

impl Registration {
    /// Builds the registration and rejects two fields that share a member.
    pub fn from_declaration(declaration: &RecordDecl<Native>) -> Result<Self> {
        let constant = Name::new(declaration.name()).constant()?;
        // `Data.define` raises at load time for a duplicate member.
        let mut scope = NameScope::new(format!("record `{constant}` members"));
        let members = field_keys(declaration)?
            .into_iter()
            .map(|key| {
                let member = member(key)?;
                scope.claim(member.as_str(), field_subject(key))?;
                Ok(member)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            constant,
            class: RecordSymbols::for_record(declaration)?.class().clone(),
            members,
            subject: format!("record `{}`", spelling(declaration.name())),
        })
    }
}

fn field_keys(declaration: &RecordDecl<Native>) -> Result<Vec<&FieldKey>> {
    Ok(match declaration {
        RecordDecl::Direct(record) => record.fields().iter().map(|field| field.key()).collect(),
        RecordDecl::Encoded(record) => record.fields().iter().map(|field| field.key()).collect(),
        _ => return unsupported("unknown record declaration"),
    })
}

fn field_subject(key: &FieldKey) -> String {
    match key {
        FieldKey::Named(name) => format!("field `{}`", spelling(name)),
        FieldKey::Position(position) => format!("field {position}"),
        _ => "field".to_owned(),
    }
}
