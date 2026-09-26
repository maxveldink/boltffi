//! Ruby objects into encoded bytes for Rust.
//!
//! Each node renders C statements that check one Ruby value and append its
//! bytes to `writer`. Containers bind each element to a `VALUE` local named
//! after the codec binder. A `Hash` needs `rb_hash_foreach`, which takes a C
//! callback, so each map node also renders one helper function.
//!
//! Encoding runs no Ruby code: every check reads the type of a value and
//! never calls a conversion method. So no `Array` or `Hash` can change
//! between the statement that writes its size and the ones that write its
//! elements.

use askama::Template;
use boltffi_binding::{
    BinderId, BuiltinType, CallbackId, ClassId, CodecWrite, CustomTypeId, ElementCount, EnumId,
    FieldKey, MapKind, Native, Op, Primitive, RecordId, ValueRef, ValueRoot, WritePlan,
};

use crate::{
    bridge::c::{Expression, Identifier, Statement},
    core::{Error, RenderContext, Result},
    target::ruby::{
        support::unsupported,
        symbol::{PrimitiveSymbols, RecordSymbols},
    },
};

#[derive(Template)]
#[template(path = "target/ruby/map_callback.c", escape = "none")]
struct MapCallbackTemplate {
    name: Identifier,
    key: Identifier,
    value: Identifier,
    write_key: Statement,
    write_value: Statement,
}

/// Renders encode statements for codec trees.
pub struct Writer<'context, 'bindings> {
    context: &'context RenderContext<'bindings, Native>,
    root: Expression,
    root_path: Vec<FieldKey>,
    helper_prefix: String,
    helpers: Vec<String>,
}

impl<'context, 'bindings> Writer<'context, 'bindings> {
    /// Creates a writer whose plans start at the C expression `root`.
    ///
    /// `helper_prefix` must be unique in the extension, because map callbacks
    /// are file-scope C functions.
    pub fn new(
        context: &'context RenderContext<'bindings, Native>,
        root: Expression,
        helper_prefix: impl Into<String>,
    ) -> Self {
        Self {
            context,
            root,
            root_path: Vec::new(),
            helper_prefix: helper_prefix.into(),
            helpers: Vec::new(),
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

    /// Returns the rendered helper functions that the statements call.
    pub fn into_helpers(self) -> Vec<String> {
        self.helpers
    }

    /// Returns the C expression for one value of the plan.
    ///
    /// A tuple element re-reads its path from the root at each use. That is
    /// safe because encoding runs no Ruby code, so no `Array` on the path can
    /// change after `boltffi_ruby_expect_tuple` checked it.
    fn reference(&self, value: &ValueRef) -> Result<Expression> {
        let (root, path) = match value.root() {
            ValueRoot::Binder(binder) => (Expression::identifier(bound(*binder)?), value.path()),
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
        path.iter().try_fold(root, |expression, key| match key {
            FieldKey::Position(position) => Ok(Expression::new(format!(
                "rb_ary_entry({expression}, {position})"
            ))),
            _ => unsupported("named codec value path"),
        })
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

fn bound(binder: BinderId) -> Result<Identifier> {
    Identifier::parse(format!("boltffi_bound_{}", binder.raw()))
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

    fn optional(
        &mut self,
        value: &ValueRef,
        binder: BinderId,
        inner: Vec<Self::Stmt>,
    ) -> Vec<Self::Stmt> {
        let statement = self.reference(value).and_then(|value| {
            let bound = bound(binder)?;
            let inner = Self::join(inner)?;
            Ok(Statement::new(format!(
                "{{\n\
                 VALUE {bound} = {value};\n\
                 if (NIL_P({bound})) {{\n\
                 boltffi_ruby_write_raw_u8(writer, 0);\n\
                 }} else {{\n\
                 boltffi_ruby_write_raw_u8(writer, 1);\n\
                 {inner}\n\
                 }}\n\
                 }}"
            )))
        });
        vec![statement]
    }

    fn sequence(
        &mut self,
        value: &ValueRef,
        _: &Op<ElementCount>,
        binder: BinderId,
        element: Vec<Self::Stmt>,
    ) -> Vec<Self::Stmt> {
        let statement = self.reference(value).and_then(|value| {
            let raw = binder.raw();
            let bound = bound(binder)?;
            let element = Self::join(element)?;
            Ok(Statement::new(format!(
                "{{\n\
                 VALUE boltffi_array_{raw} = boltffi_ruby_expect_array({value});\n\
                 long boltffi_length_{raw} = RARRAY_LEN(boltffi_array_{raw});\n\
                 boltffi_ruby_write_count(writer, boltffi_length_{raw});\n\
                 for (long boltffi_index_{raw} = 0; boltffi_index_{raw} < boltffi_length_{raw}; boltffi_index_{raw}++) {{\n\
                 VALUE {bound} = rb_ary_entry(boltffi_array_{raw}, boltffi_index_{raw});\n\
                 {element}\n\
                 }}\n\
                 }}"
            )))
        });
        vec![statement]
    }

    fn tuple(&mut self, value: &ValueRef, elements: Vec<Vec<Self::Stmt>>) -> Vec<Self::Stmt> {
        let check = self.reference(value).map(|value| {
            Statement::new(format!(
                "boltffi_ruby_expect_tuple({value}, {});",
                elements.len()
            ))
        });
        std::iter::once(check)
            .chain(elements.into_iter().flatten())
            .collect()
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
        value: &ValueRef,
        key_binder: BinderId,
        key: Vec<Self::Stmt>,
        value_binder: BinderId,
        map_value: Vec<Self::Stmt>,
    ) -> Vec<Self::Stmt> {
        let statement = self.reference(value).and_then(|value| {
            let raw = key_binder.raw();
            let callback = Identifier::parse(format!("{}_map_{raw}", self.helper_prefix))?;
            self.helpers.push(
                MapCallbackTemplate {
                    name: callback.clone(),
                    key: bound(key_binder)?,
                    value: bound(value_binder)?,
                    write_key: Self::join(key)?,
                    write_value: Self::join(map_value)?,
                }
                .render()?,
            );
            Ok(Statement::new(format!(
                "{{\n\
                 VALUE boltffi_hash_{raw} = boltffi_ruby_expect_hash({value});\n\
                 boltffi_ruby_write_count(writer, (long)RHASH_SIZE(boltffi_hash_{raw}));\n\
                 rb_hash_foreach(boltffi_hash_{raw}, {callback}, (VALUE)writer);\n\
                 }}"
            )))
        });
        vec![statement]
    }
}
