//! Which lowered shapes the Ruby target can move.
//!
//! The record and function renderers both ask this module before they emit
//! code, so a function never calls into a record the target skipped.

use std::collections::BTreeSet;

use boltffi_binding::{
    BuiltinType, CallbackId, ClassId, CodecRead, CustomTypeId, ElementCount, EnumId, MapKind,
    Native, Op, Primitive, ReadPlan, RecordDecl, RecordId,
};

use crate::core::{Error, RenderContext, Result};

/// The outermost shape of a codec tree that the target can move.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// `Option<T>`: Ruby `nil` or the value.
    Optional,
    /// Every other shape.
    Value,
}

/// Walks codec trees and records to find the first shape Ruby cannot move.
pub struct Support<'context, 'bindings> {
    context: &'context RenderContext<'bindings, Native>,
    visiting: BTreeSet<RecordId>,
}

impl<'context, 'bindings> Support<'context, 'bindings> {
    pub fn new(context: &'context RenderContext<'bindings, Native>) -> Self {
        Self {
            context,
            visiting: BTreeSet::new(),
        }
    }

    /// Checks one record and every record its fields reach.
    pub fn record(&mut self, id: RecordId) -> Result<()> {
        // A record that reaches itself (`children: Vec<Node>`) is supported when
        // its other fields are, so a record already on the stack passes here.
        if !self.visiting.insert(id) {
            return Ok(());
        }
        let result = match self.context.record(id) {
            Some(RecordDecl::Direct(_)) => Ok(()),
            Some(RecordDecl::Encoded(record)) => record
                .fields()
                .iter()
                .try_for_each(|field| field.read().render_with(self).map(drop)),
            Some(_) => unsupported("unknown record declaration"),
            None => Err(Error::UnexpectedBindingShape {
                layer: "ruby host",
                shape: "missing record declaration",
            }),
        };
        self.visiting.remove(&id);
        result
    }

    /// Checks one codec tree.
    pub fn codec(&mut self, plan: &ReadPlan) -> Result<()> {
        plan.render_with(self).map(drop)
    }
}

impl CodecRead for Support<'_, '_> {
    type Expr = Result<Shape>;

    fn primitive(&mut self, _: Primitive) -> Self::Expr {
        Ok(Shape::Value)
    }

    fn string(&mut self) -> Self::Expr {
        Ok(Shape::Value)
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
        Ok(Shape::Value)
    }

    fn direct_record(&mut self, id: RecordId) -> Self::Expr {
        self.record(id).map(|()| Shape::Value)
    }

    fn encoded_record(&mut self, id: RecordId) -> Self::Expr {
        self.record(id).map(|()| Shape::Value)
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
        // Ruby `nil` stands for `None`, so `Some(None)` would also become `nil`.
        match inner? {
            Shape::Optional => unsupported("nested optional"),
            Shape::Value => Ok(Shape::Optional),
        }
    }

    fn sequence(&mut self, _: &Op<ElementCount>, element: Self::Expr) -> Self::Expr {
        element.map(|_| Shape::Value)
    }

    fn tuple(&mut self, elements: Vec<Self::Expr>) -> Self::Expr {
        elements
            .into_iter()
            .try_for_each(|element| element.map(drop))
            .map(|()| Shape::Value)
    }

    fn result(&mut self, _: Self::Expr, _: Self::Expr) -> Self::Expr {
        unsupported("Result value")
    }

    fn map(&mut self, _: MapKind, key: Self::Expr, value: Self::Expr) -> Self::Expr {
        key?;
        value?;
        Ok(Shape::Value)
    }
}

/// Returns the typed error a renderer uses to skip one declaration.
pub fn unsupported<T>(shape: &'static str) -> Result<T> {
    Err(Error::UnsupportedTarget {
        target: "ruby",
        shape,
    })
}
