//! Free functions as Ruby module functions.
//!
//! Each function renders one C wrapper with a fixed arity. Ruby and YJIT call
//! a fixed-arity C function directly, with no argument array. The wrapper
//! converts every argument before the native call and converts the result
//! into a Ruby object after it.

use askama::Template;
use boltffi_binding::{
    DirectValueType, ErrorDecl, ExecutionDecl, FunctionDecl, IncomingParam, IntoRust, Native,
    OutOfRust, ParamDecl, ParamPlan, Primitive, Receive, ReturnPlan,
};

use crate::{
    bridge::c::{
        ArgumentList, CBridgeContract, Expression, Identifier, Parameter, Statement, TypeFragment,
    },
    core::{Emitted, Error, Result},
    target::ruby::{
        cext::{support::unsupported, symbol::PrimitiveSymbols},
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
    result: Expression,
}

/// The `rb_define_module_function` call for one rendered function.
pub struct Registration {
    pub ruby_name: RubyIdentifier,
    pub wrapper: Identifier,
    pub arity: i32,
    /// The Rust function, for name collision diagnostics.
    pub subject: String,
}

/// C code that moves one argument into native call slots.
struct Argument {
    setup: Vec<Statement>,
    values: Vec<Expression>,
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
        let conversion = ReturnConversion::from_plan(callable.returns().plan())?;
        let (call, result) = conversion.call(&native_call);
        let mut setup = Vec::new();
        for argument in arguments {
            setup.extend(argument.setup);
        }
        Ok(Self {
            wrapper: registration.wrapper,
            arity: callable.params().len(),
            setup,
            call,
            result,
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
        })
    }
}

/// How the wrapper turns the native return slot into a Ruby value.
struct ReturnConversion {
    kind: ReturnKind,
}

enum ReturnKind {
    Void,
    Direct {
        c_type: TypeFragment,
        convert: Identifier,
    },
}

impl ReturnConversion {
    fn from_plan(plan: &ReturnPlan<Native, OutOfRust>) -> Result<Self> {
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
            ReturnPlan::DirectViaReturnSlot {
                ty: DirectValueType::Enum(_),
            } => unsupported("enum return"),
            ReturnPlan::HandleViaReturnSlot { .. } => unsupported("handle return"),
            _ => unsupported("return value"),
        }
    }

    fn plain(kind: ReturnKind) -> Self {
        Self { kind }
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
        }
    }
}
