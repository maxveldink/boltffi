//! Encoded bytes from Rust into Ruby objects.
//!
//! Each node renders C statements that read from `reader` and store one
//! `VALUE` into a destination.

use boltffi_binding::{
    BuiltinType, CallbackId, ClassId, CodecRead, CustomTypeId, ElementCount, EnumId, MapKind, Op,
    Primitive, ReadPlan, RecordId,
};

use crate::{
    bridge::c::{Identifier, Statement},
    core::Result,
    target::ruby::cext::support::unsupported,
};

/// Placeholder that a parent replaces with the destination of the value.
const DESTINATION: &str = "BOLTFFI_RUBY_DESTINATION";

/// C statements that decode one value into a destination.
pub struct Decoded {
    statements: String,
}

impl Decoded {
    fn new(statements: String) -> Self {
        Self { statements }
    }

    fn assign_to(&self, destination: &str) -> String {
        self.statements.replace(DESTINATION, destination)
    }
}

/// Renders decode statements for codec trees.
pub struct Reader;

impl Reader {
    /// Renders statements that decode `plan` from `reader` into `destination`.
    pub fn decode(&mut self, plan: &ReadPlan, destination: &str) -> Result<Statement> {
        plan.render_with(self)
            .map(|decoded| Statement::new(decoded.assign_to(destination)))
    }

    fn call(function: &Identifier) -> Decoded {
        Decoded::new(format!("{DESTINATION} = {function}(reader);"))
    }
}

impl CodecRead for Reader {
    type Expr = Result<Decoded>;

    fn primitive(&mut self, _: Primitive) -> Self::Expr {
        unsupported("encoded scalar")
    }

    fn string(&mut self) -> Self::Expr {
        Ok(Self::call(&Identifier::parse("boltffi_ruby_read_string")?))
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
        Ok(Self::call(&Identifier::parse("boltffi_ruby_read_binary")?))
    }

    fn direct_record(&mut self, _: RecordId) -> Self::Expr {
        unsupported("record")
    }

    fn encoded_record(&mut self, _: RecordId) -> Self::Expr {
        unsupported("record")
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

    fn optional(&mut self, _: Self::Expr) -> Self::Expr {
        unsupported("optional value")
    }

    fn sequence(&mut self, _: &Op<ElementCount>, _: Self::Expr) -> Self::Expr {
        unsupported("sequence")
    }

    fn tuple(&mut self, _: Vec<Self::Expr>) -> Self::Expr {
        unsupported("tuple")
    }

    fn result(&mut self, _: Self::Expr, _: Self::Expr) -> Self::Expr {
        unsupported("Result value")
    }

    fn map(&mut self, _: MapKind, _: Self::Expr, _: Self::Expr) -> Self::Expr {
        unsupported("map")
    }
}
