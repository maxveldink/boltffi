//! Ruby objects into encoded bytes for Rust.
//!
//! Each node renders C statements that check one Ruby value and append its
//! bytes to `writer`.
//!
//! Encoding runs no Ruby code: every check reads the type of a value and
//! never calls a conversion method.

use boltffi_binding::{
    BinderId, BuiltinType, CallbackId, ClassId, CodecWrite, CustomTypeId, ElementCount, EnumId,
    FieldKey, MapKind, Native, Op, Primitive, RecordId, ValueRef, ValueRoot, WritePlan,
};

use crate::{
    bridge::c::{Expression, Identifier, Statement},
    core::{Error, RenderContext, Result},
    target::ruby::cext::{
        support::unsupported,
        symbol::{PrimitiveSymbols, RecordSymbols},
    },
};

/// Renders encode statements for codec trees.
pub struct Writer<'context, 'bindings> {
    context: &'context RenderContext<'bindings, Native>,
    root: Expression,
    root_path: Vec<FieldKey>,
}

impl<'context, 'bindings> Writer<'context, 'bindings> {
    /// Creates a writer whose plans start at the C expression `root`.
    pub fn new(context: &'context RenderContext<'bindings, Native>, root: Expression) -> Self {
        Self {
            context,
            root,
            root_path: Vec::new(),
        }
    }

    /// Declares that `root` already resolves this path of the plan's value.
    ///
    /// A record field plan starts at `self.field`; the record writer binds the
    /// field to a local first, so the writer strips `field` from the path.
    pub fn root_path(mut self, path: Vec<FieldKey>) -> Self {
        self.root_path = path;
        self
    }

    /// Renders statements that append `plan` to `writer`.
    pub fn encode(&mut self, plan: &WritePlan) -> Result<Statement> {
        Self::join(plan.render_with(self))
    }

    /// Returns the C expression for one value of the plan.
    fn reference(&self, value: &ValueRef) -> Result<Expression> {
        let (root, path) = match value.root() {
            ValueRoot::SelfValue | ValueRoot::Named(_) | ValueRoot::Local(_) => {
                let path = value.path().strip_prefix(self.root_path.as_slice()).ok_or(
                    Error::UnsupportedTarget {
                        target: "ruby",
                        shape: "codec value path",
                    },
                )?;
                (self.root.clone(), path)
            }
            _ => return unsupported("codec value root"),
        };
        if !path.is_empty() {
            return unsupported("codec value path");
        }
        Ok(root)
    }

    fn call(&self, function: Result<Identifier>, value: &ValueRef) -> Vec<Result<Statement>> {
        vec![function.and_then(|function| {
            let value = self.reference(value)?;
            Ok(Statement::new(format!("{function}(writer, {value});")))
        })]
    }

    fn join(statements: Vec<Result<Statement>>) -> Result<Statement> {
        statements
            .into_iter()
            .map(|statement| statement.map(|statement| statement.to_string()))
            .collect::<Result<Vec<_>>>()
            .map(|statements| Statement::new(statements.join("\n")))
    }
}

impl CodecWrite for Writer<'_, '_> {
    type Stmt = Result<Statement>;

    fn primitive(&mut self, primitive: Primitive, value: &ValueRef) -> Vec<Self::Stmt> {
        self.call(PrimitiveSymbols::new(primitive).writer(), value)
    }

    fn string(&mut self, value: &ValueRef) -> Vec<Self::Stmt> {
        self.call(Identifier::parse("boltffi_ruby_write_string"), value)
    }

    fn utf8_string(&mut self, _: &ValueRef) -> Vec<Self::Stmt> {
        vec![unsupported("unframed UTF-8 string")]
    }

    fn raw_bytes(&mut self, _: &ValueRef) -> Vec<Self::Stmt> {
        vec![unsupported("unframed byte buffer")]
    }

    fn interned_string(&mut self, _: &[String], _: &ValueRef) -> Vec<Self::Stmt> {
        vec![unsupported("interned string")]
    }

    fn bytes(&mut self, value: &ValueRef) -> Vec<Self::Stmt> {
        self.call(Identifier::parse("boltffi_ruby_write_binary"), value)
    }

    fn direct_record(&mut self, id: RecordId, value: &ValueRef) -> Vec<Self::Stmt> {
        let writer = RecordSymbols::new(id, self.context).map(|symbols| symbols.writer().clone());
        self.call(writer, value)
    }

    fn encoded_record(&mut self, id: RecordId, value: &ValueRef) -> Vec<Self::Stmt> {
        self.direct_record(id, value)
    }

    fn c_style_enum(&mut self, _: EnumId, _: &ValueRef) -> Vec<Self::Stmt> {
        vec![unsupported("enum")]
    }

    fn data_enum(&mut self, _: EnumId, _: &ValueRef) -> Vec<Self::Stmt> {
        vec![unsupported("enum")]
    }

    fn class_handle(&mut self, _: ClassId, _: &ValueRef) -> Vec<Self::Stmt> {
        vec![unsupported("class handle")]
    }

    fn callback_handle(&mut self, _: CallbackId, _: &ValueRef) -> Vec<Self::Stmt> {
        vec![unsupported("callback handle")]
    }

    fn custom<F>(&mut self, _: CustomTypeId, _: &ValueRef, _: F) -> Vec<Self::Stmt>
    where
        F: FnOnce(&mut Self, &ValueRef) -> Vec<Self::Stmt>,
    {
        vec![unsupported("custom type")]
    }

    fn builtin(&mut self, _: BuiltinType, _: &ValueRef) -> Vec<Self::Stmt> {
        vec![unsupported("builtin type")]
    }

    fn optional(&mut self, _: &ValueRef, _: BinderId, _: Vec<Self::Stmt>) -> Vec<Self::Stmt> {
        vec![unsupported("optional value")]
    }

    fn sequence(
        &mut self,
        _: &ValueRef,
        _: &Op<ElementCount>,
        _: BinderId,
        _: Vec<Self::Stmt>,
    ) -> Vec<Self::Stmt> {
        vec![unsupported("sequence")]
    }

    fn tuple(&mut self, _: &ValueRef, _: Vec<Vec<Self::Stmt>>) -> Vec<Self::Stmt> {
        vec![unsupported("tuple")]
    }

    fn result(
        &mut self,
        _: &ValueRef,
        _: BinderId,
        _: Vec<Self::Stmt>,
        _: Vec<Self::Stmt>,
    ) -> Vec<Self::Stmt> {
        vec![unsupported("Result value")]
    }

    fn map(
        &mut self,
        _: MapKind,
        _: &ValueRef,
        _: BinderId,
        _: Vec<Self::Stmt>,
        _: BinderId,
        _: Vec<Self::Stmt>,
    ) -> Vec<Self::Stmt> {
        vec![unsupported("map")]
    }
}
