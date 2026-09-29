//! Free functions as Ruby module functions.
//!
//! Each function renders one C wrapper with a fixed arity. Ruby and YJIT call
//! a fixed-arity C function directly, with no argument array. The wrapper
//! converts every argument before the native call and converts the result
//! into a Ruby object after it.

use askama::Template;
use boltffi_binding::{
    DirectValueType, ErrorDecl, ExecutionDecl, FunctionDecl, IncomingParam, IntoRust, Native,
    OutOfRust, ParamDecl, ParamPlan, Primitive, Receive, ReturnPlan, native,
};

use crate::{
    bridge::c::{
        ArgumentList, CBridgeContract, Expression, Identifier, Parameter, Statement, Type,
        TypeFragment,
    },
    core::{Emitted, Error, Result},
    target::ruby::{
        cext::{
            codec::{read::Reader, write::Writer},
            support::unsupported,
            symbol::PrimitiveSymbols,
        },
        name_style::{Name, spelling},
        syntax::Identifier as RubyIdentifier,
    },
};

/// Ruby registers C methods with a fixed arity of at most 15 arguments.
const MAX_FIXED_ARITY: usize = 15;

#[derive(Template)]
#[template(path = "target/ruby/function.c", escape = "none")]
struct FunctionTemplate<'function> {
    function: &'function Function,
}

/// One rendered module function.
pub struct Function {
    wrapper: Identifier,
    arity: usize,
    setup: Vec<Statement>,
    call: Statement,
    cleanup: Vec<Statement>,
    check: Option<Statement>,
    result: Expression,
    decoder: Option<Decoder>,
}

/// The `rb_define_module_function` call for one rendered function.
pub struct Registration {
    pub ruby_name: RubyIdentifier,
    pub wrapper: Identifier,
    pub arity: i32,
    /// The Rust function, for name collision diagnostics.
    pub subject: String,
}

/// A C function that decodes the buffer a function returns.
struct Decoder {
    name: Identifier,
    body: Statement,
}

/// C code that moves one argument into native call slots.
#[derive(Default)]
struct Argument {
    setup: Vec<Statement>,
    values: Vec<Expression>,
    cleanup: Vec<Statement>,
    /// Rust decodes this argument from encoded bytes, so decoding can fail.
    encoded: bool,
}

impl Function {
    pub fn from_declaration(
        declaration: &FunctionDecl<Native>,
        bridge: &CBridgeContract,
    ) -> Result<Self> {
        let callable = declaration.callable();
        if matches!(callable.execution(), ExecutionDecl::Asynchronous(_)) {
            return unsupported("async function");
        }
        if !matches!(callable.error(), ErrorDecl::None(_)) {
            return unsupported("fallible function");
        }
        if callable.receiver().is_some() {
            return unsupported("function receiver");
        }
        let registration = Registration::from_declaration(declaration)?;
        let abi = bridge.function(declaration.symbol())?;
        let variadic = callable.params().len() > MAX_FIXED_ARITY;
        let mut abi_params = abi.params().iter();
        let arguments = callable
            .params()
            .iter()
            .enumerate()
            .map(|(index, param)| {
                let value = Expression::new(if variadic {
                    format!("argv[{index}]")
                } else {
                    format!("boltffi_arg_{index}")
                });
                Argument::from_param(ArgumentSlot { index, value }, param, &mut abi_params)
            })
            .collect::<Result<Vec<_>>>()?;
        if abi_params.next().is_some() {
            return Err(Error::BrokenBridgeContract {
                bridge: "c",
                invariant: "Ruby wrapper passed fewer arguments than the C function takes",
            });
        }
        let native_call = Expression::call(
            Identifier::parse(abi.name())?,
            ArgumentList::from_iter(
                arguments
                    .iter()
                    .flat_map(|argument| argument.values.iter().cloned()),
            ),
        );
        let conversion = ReturnConversion::from_plan(
            callable.returns().plan(),
            &registration.wrapper,
            abi.returns(),
        )?;
        let (call, result) = conversion.call(&native_call);
        let check = arguments
            .iter()
            .any(|argument| argument.encoded)
            .then(|| conversion.check());
        let mut setup = Vec::new();
        let mut cleanup = Vec::new();
        for argument in arguments {
            setup.extend(argument.setup);
            cleanup.extend(argument.cleanup);
        }
        Ok(Self {
            wrapper: registration.wrapper,
            arity: callable.params().len(),
            setup,
            call,
            cleanup,
            check,
            result,
            decoder: conversion.decoder,
        })
    }

    pub fn render(&self) -> Result<Emitted> {
        Ok(Emitted::primary(
            FunctionTemplate { function: self }.render()?,
        ))
    }

    fn variadic(&self) -> bool {
        self.arity > MAX_FIXED_ARITY
    }
}

impl Registration {
    pub fn from_declaration(declaration: &FunctionDecl<Native>) -> Result<Self> {
        let arity = declaration.callable().params().len();
        Ok(Self {
            ruby_name: Name::new(declaration.name()).module_function()?,
            wrapper: Identifier::parse(format!(
                "boltffi_ruby_fn_{}",
                declaration.symbol().name().as_str()
            ))?,
            arity: if arity > MAX_FIXED_ARITY {
                -1
            } else {
                arity as i32
            },
            subject: format!("function `{}`", spelling(declaration.name())),
        })
    }
}

/// Where one Ruby argument comes from inside the wrapper.
struct ArgumentSlot {
    index: usize,
    value: Expression,
}

impl Argument {
    fn from_param<'abi>(
        slot: ArgumentSlot,
        param: &ParamDecl<Native, IntoRust>,
        abi: &mut impl Iterator<Item = &'abi Parameter>,
    ) -> Result<Self> {
        let plan = match param.payload() {
            IncomingParam::Value(plan) => plan,
            IncomingParam::Closure(_) => return unsupported("closure parameter"),
        };
        let mut next_abi = || {
            abi.next().ok_or(Error::BrokenBridgeContract {
                bridge: "c",
                invariant: "Ruby wrapper passed more arguments than the C function takes",
            })
        };
        match plan {
            ParamPlan::Direct {
                ty: DirectValueType::Primitive(primitive),
                receive: Receive::ByValue,
            } => {
                next_abi()?;
                Self::primitive(&slot, *primitive)
            }
            ParamPlan::Encoded {
                codec,
                shape: native::BufferShape::Slice,
                receive: Receive::ByValue | Receive::ByRef,
                ..
            } => {
                next_abi()?;
                next_abi()?;
                let statements = Writer::new(slot.value.clone()).encode(codec)?;
                Ok(Self::encoded(&slot, statements))
            }
            ParamPlan::Direct {
                ty: DirectValueType::Enum(_),
                ..
            } => unsupported("enum parameter"),
            ParamPlan::Handle { .. } => unsupported("handle parameter"),
            ParamPlan::Direct {
                receive: Receive::ByMutRef,
                ..
            }
            | ParamPlan::Encoded {
                receive: Receive::ByMutRef,
                ..
            }
            | ParamPlan::DirectVec {
                receive: Receive::ByMutRef,
                ..
            } => unsupported("mutable borrowed parameter"),
            _ => unsupported("parameter"),
        }
    }

    fn primitive(slot: &ArgumentSlot, primitive: Primitive) -> Result<Self> {
        let symbols = PrimitiveSymbols::new(primitive);
        let local = format!("boltffi_value_{}", slot.index);
        Ok(Self {
            setup: vec![Statement::new(format!(
                "{} {local} = {}({});",
                symbols.c_type()?,
                symbols.ruby_to_c()?,
                slot.value
            ))],
            values: vec![Expression::new(local)],
            ..Self::default()
        })
    }

    fn encoded(slot: &ArgumentSlot, statements: Statement) -> Self {
        let writer = format!("boltffi_writer_{}", slot.index);
        Self {
            setup: vec![
                Statement::new(format!("boltffi_ruby_writer {writer};")),
                Statement::new(format!("boltffi_ruby_writer_init(&{writer});")),
                Statement::new(format!(
                    "{{\nboltffi_ruby_writer *writer = &{writer};\n{statements}\n}}"
                )),
            ],
            values: Self::writer_values(&writer),
            cleanup: vec![Statement::new(format!("RB_GC_GUARD({writer}.heap);"))],
            encoded: true,
        }
    }

    fn writer_values(writer: &str) -> Vec<Expression> {
        vec![
            Expression::new(format!("{writer}.ptr")),
            Expression::new(format!("{writer}.len")),
        ]
    }
}

/// How the wrapper turns the native return slot into a Ruby value.
struct ReturnConversion {
    kind: ReturnKind,
    decoder: Option<Decoder>,
}

enum ReturnKind {
    Void,
    Direct {
        c_type: TypeFragment,
        convert: Identifier,
    },
    Owned {
        decoder: Identifier,
    },
}

impl ReturnConversion {
    fn from_plan(
        plan: &ReturnPlan<Native, OutOfRust>,
        wrapper: &Identifier,
        abi_return: &Type,
    ) -> Result<Self> {
        let decoder = || Identifier::parse(format!("{wrapper}_decode"));
        match plan {
            ReturnPlan::Void => Ok(Self::plain(ReturnKind::Void)),
            ReturnPlan::DirectViaReturnSlot {
                ty: DirectValueType::Primitive(primitive),
            } => {
                let symbols = PrimitiveSymbols::new(*primitive);
                Ok(Self::plain(ReturnKind::Direct {
                    c_type: symbols.c_type()?,
                    convert: symbols.c_to_ruby()?,
                }))
            }
            ReturnPlan::EncodedViaReturnSlot {
                codec,
                shape: native::BufferShape::Buffer,
                ..
            } => {
                let body = Reader.decode(codec, "boltffi_value")?;
                Self::owned(decoder()?, body, abi_return)
            }
            ReturnPlan::DirectViaReturnSlot {
                ty: DirectValueType::Enum(_),
            } => unsupported("enum return"),
            ReturnPlan::HandleViaReturnSlot { .. } => unsupported("handle return"),
            _ => unsupported("return value"),
        }
    }

    fn plain(kind: ReturnKind) -> Self {
        Self {
            kind,
            decoder: None,
        }
    }

    fn owned(name: Identifier, body: Statement, abi_return: &Type) -> Result<Self> {
        if !matches!(abi_return, Type::Buffer) {
            return Err(Error::BrokenBridgeContract {
                bridge: "c",
                invariant: "encoded return does not use a FfiBuf_u8 return slot",
            });
        }
        Ok(Self {
            kind: ReturnKind::Owned {
                decoder: name.clone(),
            },
            decoder: Some(Decoder { name, body }),
        })
    }

    /// Returns the statement that raises when Rust rejected an argument.
    fn check(&self) -> Statement {
        Statement::new(match self.kind {
            ReturnKind::Owned { .. } => "boltffi_ruby_check_arguments(&boltffi_result);",
            ReturnKind::Void | ReturnKind::Direct { .. } => "boltffi_ruby_check_arguments(NULL);",
        })
    }

    /// Returns the call statement and the Ruby result expression.
    fn call(&self, native_call: &Expression) -> (Statement, Expression) {
        match &self.kind {
            ReturnKind::Void => (
                Statement::new(format!("{native_call};")),
                Expression::new("Qnil"),
            ),
            ReturnKind::Direct { c_type, convert } => (
                Statement::new(format!("{c_type} boltffi_result = {native_call};")),
                Expression::new(format!("{convert}(boltffi_result)")),
            ),
            ReturnKind::Owned { decoder } => (
                Statement::new(format!("FfiBuf_u8 boltffi_result = {native_call};")),
                Expression::new(format!(
                    "boltffi_ruby_decode_owned(boltffi_result, {decoder})"
                )),
            ),
        }
    }
}
