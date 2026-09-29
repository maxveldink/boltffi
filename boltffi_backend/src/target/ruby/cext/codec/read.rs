//! Encoded bytes from Rust into Ruby objects.
//!
//! Each node renders C statements that read from `reader` and store one
//! `VALUE` into a destination. A container embeds its children's statements
//! with its own destination, so any nesting of optionals, arrays, hashes,
//! tuples, and records renders as straight-line C.

use std::collections::BTreeSet;

use boltffi_binding::{
    BuiltinType, CallbackId, ClassId, CodecRead, CustomTypeId, ElementCount, EnumId, MapKind,
    Native, Op, Primitive, ReadPlan, RecordDecl, RecordId,
};

use crate::{
    bridge::c::{Identifier, Statement},
    core::{RenderContext, Result},
    target::ruby::cext::{
        support::unsupported,
        symbol::{PrimitiveSymbols, RecordSymbols},
    },
};

/// Placeholder that a parent replaces with the destination of the value.
const DESTINATION: &str = "BOLTFFI_RUBY_DESTINATION";

/// C statements that decode one value into a destination.
pub struct Decoded {
    statements: String,
    minimum_size: usize,
}

impl Decoded {
    fn new(statements: String, minimum_size: usize) -> Self {
        Self {
            statements,
            minimum_size,
        }
    }

    fn assign_to(&self, destination: &str) -> String {
        self.statements.replace(DESTINATION, destination)
    }
}

/// Renders decode statements for codec trees.
pub struct Reader<'context, 'bindings> {
    context: &'context RenderContext<'bindings, Native>,
    next_local: usize,
}

impl<'context, 'bindings> Reader<'context, 'bindings> {
    pub fn new(context: &'context RenderContext<'bindings, Native>) -> Self {
        Self {
            context,
            next_local: 0,
        }
    }

    /// Renders statements that decode `plan` from `reader` into `destination`.
    pub fn decode(&mut self, plan: &ReadPlan, destination: &str) -> Result<Statement> {
        plan.render_with(self)
            .map(|decoded| Statement::new(decoded.assign_to(destination)))
    }

    fn local(&mut self, stem: &str) -> String {
        let name = format!("boltffi_{stem}_{}", self.next_local);
        self.next_local += 1;
        name
    }

    fn call(function: &Identifier, minimum_size: usize) -> Decoded {
        Decoded::new(format!("{DESTINATION} = {function}(reader);"), minimum_size)
    }
}

impl CodecRead for Reader<'_, '_> {
    type Expr = Result<Decoded>;

    fn primitive(&mut self, primitive: Primitive) -> Self::Expr {
        let symbols = PrimitiveSymbols::new(primitive);
        Ok(Self::call(&symbols.reader()?, symbols.wire_size()))
    }

    fn string(&mut self) -> Self::Expr {
        Ok(Self::call(
            &Identifier::parse("boltffi_ruby_read_string")?,
            4,
        ))
    }

    fn utf8_string(&mut self) -> Self::Expr {
        unsupported("unframed UTF-8 string")
    }

    fn raw_bytes(&mut self) -> Self::Expr {
        unsupported("unframed byte buffer")
    }

    fn interned_string(&mut self, _: &[String]) -> Self::Expr {
        unsupported("interned string")
    }

    fn bytes(&mut self) -> Self::Expr {
        Ok(Self::call(
            &Identifier::parse("boltffi_ruby_read_binary")?,
            4,
        ))
    }

    fn direct_record(&mut self, id: RecordId) -> Self::Expr {
        let symbols = RecordSymbols::new(id, self.context)?;
        let minimum_size = MinimumSize::new(self.context).record(id);
        Ok(Self::call(symbols.reader(), minimum_size))
    }

    fn encoded_record(&mut self, id: RecordId) -> Self::Expr {
        self.direct_record(id)
    }

    fn c_style_enum(&mut self, _: EnumId) -> Self::Expr {
        unsupported("enum")
    }

    fn data_enum(&mut self, _: EnumId) -> Self::Expr {
        unsupported("enum")
    }

    fn class_handle(&mut self, _: ClassId) -> Self::Expr {
        unsupported("class handle")
    }

    fn callback_handle(&mut self, _: CallbackId) -> Self::Expr {
        unsupported("callback handle")
    }

    fn custom(&mut self, _: CustomTypeId, _: Self::Expr) -> Self::Expr {
        unsupported("custom type")
    }

    fn builtin(&mut self, _: BuiltinType) -> Self::Expr {
        unsupported("builtin type")
    }

    fn optional(&mut self, inner: Self::Expr) -> Self::Expr {
        let inner = inner?;
        Ok(Decoded::new(
            format!(
                "if (boltffi_ruby_read_option_tag(reader)) {{\n{}\n}} else {{\n{DESTINATION} = Qnil;\n}}",
                inner.assign_to(DESTINATION)
            ),
            1,
        ))
    }

    fn sequence(&mut self, _: &Op<ElementCount>, element: Self::Expr) -> Self::Expr {
        let element = element?;
        let count = self.local("count");
        let array = self.local("array");
        let index = self.local("index");
        let item = self.local("item");
        Ok(Decoded::new(
            format!(
                "{{\n\
                 long {count} = boltffi_ruby_read_count(reader, {minimum});\n\
                 VALUE {array} = rb_ary_new_capa({count});\n\
                 for (long {index} = 0; {index} < {count}; {index}++) {{\n\
                 VALUE {item} = Qnil;\n\
                 {element}\n\
                 rb_ary_push({array}, {item});\n\
                 }}\n\
                 {DESTINATION} = {array};\n\
                 }}",
                minimum = element.minimum_size,
                element = element.assign_to(&item),
            ),
            4,
        ))
    }

    fn tuple(&mut self, elements: Vec<Self::Expr>) -> Self::Expr {
        let elements = elements.into_iter().collect::<Result<Vec<_>>>()?;
        let tuple = self.local("tuple");
        let item = self.local("item");
        let minimum_size = elements.iter().map(|element| element.minimum_size).sum();
        let body = elements
            .iter()
            .map(|element| {
                format!(
                    "{}\nrb_ary_push({tuple}, {item});",
                    element.assign_to(&item)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        Ok(Decoded::new(
            format!(
                "{{\n\
                 VALUE {tuple} = rb_ary_new_capa({len});\n\
                 VALUE {item} = Qnil;\n\
                 {body}\n\
                 {DESTINATION} = {tuple};\n\
                 }}",
                len = elements.len(),
            ),
            minimum_size,
        ))
    }

    fn result(&mut self, _: Self::Expr, _: Self::Expr) -> Self::Expr {
        unsupported("Result value")
    }

    fn map(&mut self, _: MapKind, key: Self::Expr, value: Self::Expr) -> Self::Expr {
        let key = key?;
        let value = value?;
        let count = self.local("count");
        let hash = self.local("hash");
        let index = self.local("index");
        let key_local = self.local("key");
        let value_local = self.local("value");
        Ok(Decoded::new(
            format!(
                "{{\n\
                 long {count} = boltffi_ruby_read_count(reader, {minimum});\n\
                 VALUE {hash} = rb_hash_new_capa({count});\n\
                 for (long {index} = 0; {index} < {count}; {index}++) {{\n\
                 VALUE {key_local} = Qnil;\n\
                 VALUE {value_local} = Qnil;\n\
                 {key}\n\
                 {value}\n\
                 rb_hash_aset({hash}, {key_local}, {value_local});\n\
                 }}\n\
                 {DESTINATION} = {hash};\n\
                 }}",
                minimum = key.minimum_size + value.minimum_size,
                key = key.assign_to(&key_local),
                value = value.assign_to(&value_local),
            ),
            4,
        ))
    }
}

/// The fewest bytes that one encoded value takes.
///
/// A count that claims more elements than the rest of the buffer can hold is
/// malformed. The reader rejects it before Ruby allocates the container.
struct MinimumSize<'context, 'bindings> {
    context: &'context RenderContext<'bindings, Native>,
    visiting: BTreeSet<RecordId>,
}

impl<'context, 'bindings> MinimumSize<'context, 'bindings> {
    fn new(context: &'context RenderContext<'bindings, Native>) -> Self {
        Self {
            context,
            visiting: BTreeSet::new(),
        }
    }

    fn record(&mut self, id: RecordId) -> usize {
        // A record reaches itself only through an optional or a sequence, whose
        // own prefix is the minimum, so a record on the stack adds nothing.
        if !self.visiting.insert(id) {
            return 0;
        }
        let size = match self.context.record(id) {
            Some(RecordDecl::Direct(record)) => record.layout().size().get() as usize,
            Some(RecordDecl::Encoded(record)) => record
                .fields()
                .iter()
                .map(|field| field.read().render_with(self))
                .sum(),
            _ => 0,
        };
        self.visiting.remove(&id);
        size
    }
}

impl CodecRead for MinimumSize<'_, '_> {
    type Expr = usize;

    fn primitive(&mut self, primitive: Primitive) -> Self::Expr {
        PrimitiveSymbols::new(primitive).wire_size()
    }

    fn string(&mut self) -> Self::Expr {
        4
    }

    fn utf8_string(&mut self) -> Self::Expr {
        0
    }

    fn raw_bytes(&mut self) -> Self::Expr {
        0
    }

    fn interned_string(&mut self, _: &[String]) -> Self::Expr {
        0
    }

    fn bytes(&mut self) -> Self::Expr {
        4
    }

    fn direct_record(&mut self, id: RecordId) -> Self::Expr {
        self.record(id)
    }

    fn encoded_record(&mut self, id: RecordId) -> Self::Expr {
        self.record(id)
    }

    fn c_style_enum(&mut self, _: EnumId) -> Self::Expr {
        0
    }

    fn data_enum(&mut self, _: EnumId) -> Self::Expr {
        0
    }

    fn class_handle(&mut self, _: ClassId) -> Self::Expr {
        0
    }

    fn callback_handle(&mut self, _: CallbackId) -> Self::Expr {
        0
    }

    fn custom(&mut self, _: CustomTypeId, _: Self::Expr) -> Self::Expr {
        0
    }

    fn builtin(&mut self, _: BuiltinType) -> Self::Expr {
        0
    }

    fn optional(&mut self, _: Self::Expr) -> Self::Expr {
        1
    }

    fn sequence(&mut self, _: &Op<ElementCount>, _: Self::Expr) -> Self::Expr {
        4
    }

    fn tuple(&mut self, elements: Vec<Self::Expr>) -> Self::Expr {
        elements.into_iter().sum()
    }

    fn result(&mut self, _: Self::Expr, _: Self::Expr) -> Self::Expr {
        1
    }

    fn map(&mut self, _: MapKind, _: Self::Expr, _: Self::Expr) -> Self::Expr {
        4
    }
}
