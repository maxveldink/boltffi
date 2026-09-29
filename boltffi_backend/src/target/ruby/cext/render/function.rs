//! Free functions as Ruby module functions.
//!
//! Each function renders one C wrapper with a fixed arity. Ruby and YJIT call
//! a fixed-arity C function directly, with no argument array. The wrapper
//! converts every argument before the native call and converts the result
//! into a Ruby object after it.

use askama::Template;
use boltffi_binding::{
    DirectValueType, DirectVectorElementType, ErrorDecl, ExecutionDecl, FunctionDecl,
    IncomingParam, IntoRust, Native, OutOfRust, ParamDecl, ParamPlan, Primitive, Receive, RecordId,
    ReturnPlan, native,
};

use crate::{
    bridge::c::{
        ArgumentList, CBridgeContract, Expression, Identifier, Parameter, Statement, Type,
        TypeFragment,
    },
    core::{Emitted, Error, RenderContext, Result},
    target::ruby::{
        cext::{
            codec::{read::Reader, write::Writer},
            support::{Support, unsupported},
            symbol::{PrimitiveSymbols, RecordSymbols},
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
    helpers: Vec<String>,
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
    helpers: Vec<String>,
    /// Rust decodes this argument from encoded bytes, so decoding can fail.
    ///
    /// Rust also checks the other argument kinds, but those checks cannot
    /// fail here: the wrapper passes a valid pointer for a record passed by
    /// reference, and an exact byte count for a vector of records.
    encoded: bool,
}

impl Function {
    pub fn from_declaration(
        declaration: &FunctionDecl<Native>,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
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
                Argument::from_param(
                    ArgumentSlot {
                        index,
                        value,
                        wrapper: &registration.wrapper,
                    },
                    param,
                    &mut abi_params,
                    bridge,
                    context,
                )
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
            bridge,
            context,
        )?;
        let (call, result) = conversion.call(&native_call);
        let check = arguments
            .iter()
            .any(|argument| argument.encoded)
            .then(|| conversion.check());
        let mut setup = Vec::new();
        let mut cleanup = Vec::new();
        let mut helpers = Vec::new();
        for argument in arguments {
            setup.extend(argument.setup);
            cleanup.extend(argument.cleanup);
            helpers.extend(argument.helpers);
        }
        Ok(Self {
            wrapper: registration.wrapper,
            arity: callable.params().len(),
            setup,
            call,
            cleanup,
            check,
            result,
            helpers,
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
struct ArgumentSlot<'wrapper> {
    index: usize,
    value: Expression,
    wrapper: &'wrapper Identifier,
}

impl Argument {
    fn from_param<'abi>(
        slot: ArgumentSlot<'_>,
        param: &ParamDecl<Native, IntoRust>,
        abi: &mut impl Iterator<Item = &'abi Parameter>,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
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
                ty: DirectValueType::Record(record),
                receive: receive @ (Receive::ByValue | Receive::ByRef),
            } => {
                let by_pointer = matches!(next_abi()?.ty(), Type::ConstPointer(_));
                if by_pointer != matches!(receive, Receive::ByRef) {
                    return Err(Error::BrokenBridgeContract {
                        bridge: "c",
                        invariant: "direct record parameter passing does not match its C type",
                    });
                }
                Self::direct_record(&slot, *record, by_pointer, bridge, context)
            }
            ParamPlan::Encoded {
                codec,
                shape: native::BufferShape::Slice,
                receive: Receive::ByValue | Receive::ByRef,
                ..
            } => {
                next_abi()?;
                next_abi()?;
                Support::new(context).codec(&codec.read_plan())?;
                let helper_prefix = format!("{}_arg{}", slot.wrapper, slot.index);
                let mut writer = Writer::new(context, slot.value.clone(), helper_prefix);
                let statements = writer.encode(codec)?;
                Ok(Self::encoded(&slot, statements, writer.into_helpers()))
            }
            ParamPlan::ScalarOption { primitive } => {
                next_abi()?;
                next_abi()?;
                Self::scalar_option(&slot, *primitive)
            }
            ParamPlan::DirectVec {
                element,
                receive: Receive::ByValue | Receive::ByRef,
            } => {
                next_abi()?;
                next_abi()?;
                Self::direct_vector(&slot, element, bridge, context)
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

    fn primitive(slot: &ArgumentSlot<'_>, primitive: Primitive) -> Result<Self> {
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

    fn direct_record(
        slot: &ArgumentSlot<'_>,
        record: RecordId,
        by_pointer: bool,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let symbols = RecordSymbols::new(record, context)?;
        let c_type = direct_record_type(record, bridge)?;
        let local = format!("boltffi_value_{}", slot.index);
        Ok(Self {
            setup: vec![Statement::new(format!(
                "{c_type} {local} = {}({});",
                symbols.unboxer(),
                slot.value
            ))],
            values: vec![Expression::new(if by_pointer {
                format!("&{local}")
            } else {
                local
            })],
            ..Self::default()
        })
    }

    fn encoded(slot: &ArgumentSlot<'_>, statements: Statement, helpers: Vec<String>) -> Self {
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
            helpers,
            encoded: true,
        }
    }

    fn scalar_option(slot: &ArgumentSlot<'_>, primitive: Primitive) -> Result<Self> {
        let write = PrimitiveSymbols::new(primitive).writer()?;
        let writer = format!("boltffi_writer_{}", slot.index);
        let value = &slot.value;
        Ok(Self {
            setup: vec![
                Statement::new(format!("boltffi_ruby_writer {writer};")),
                Statement::new(format!("boltffi_ruby_writer_init(&{writer});")),
                Statement::new(format!(
                    "if (NIL_P({value})) {{\n\
                     boltffi_ruby_write_raw_u8(&{writer}, 0);\n\
                     }} else {{\n\
                     boltffi_ruby_write_raw_u8(&{writer}, 1);\n\
                     {write}(&{writer}, {value});\n\
                     }}"
                )),
            ],
            values: Self::writer_values(&writer),
            cleanup: vec![Statement::new(format!("RB_GC_GUARD({writer}.heap);"))],
            helpers: Vec::new(),
            encoded: true,
        })
    }

    fn writer_values(writer: &str) -> Vec<Expression> {
        vec![
            Expression::new(format!("{writer}.ptr")),
            Expression::new(format!("{writer}.len")),
        ]
    }

    fn direct_vector(
        slot: &ArgumentSlot<'_>,
        element: &DirectVectorElementType,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        // Argument locals start with the argument's name, so they never
        // collide with the codec's locals, which end in a nesting depth.
        let local = format!("boltffi_arg_{}", slot.index);
        let (c_type, convert, values) = match element {
            DirectVectorElementType::Primitive(primitive) => {
                let symbols = PrimitiveSymbols::new(primitive.primitive());
                (
                    symbols.c_type()?,
                    symbols.ruby_to_c()?,
                    vec![
                        Expression::new(format!("{local}_items")),
                        Expression::new(format!("(uintptr_t){local}_count")),
                    ],
                )
            }
            DirectVectorElementType::Record(record) => {
                let c_type = direct_record_type(*record, bridge)?;
                let values = vec![
                    Expression::new(format!("(const uint8_t *){local}_items")),
                    Expression::new(format!("(uintptr_t){local}_count * sizeof({c_type})")),
                ];
                (
                    c_type,
                    RecordSymbols::new(*record, context)?.unboxer().clone(),
                    values,
                )
            }
            _ => return unsupported("direct vector element"),
        };
        Ok(Self {
            setup: vec![
                Statement::new(format!(
                    "VALUE {local}_array = boltffi_ruby_expect_array({});",
                    slot.value
                )),
                Statement::new(format!("long {local}_count = RARRAY_LEN({local}_array);")),
                Statement::new(format!("VALUE {local}_storage = 0;")),
                Statement::new(format!(
                    "{c_type} *{local}_items = RB_ALLOCV_N({c_type}, {local}_storage, {local}_count);"
                )),
                Statement::new(format!(
                    "for (long {local}_index = 0; {local}_index < {local}_count; {local}_index++) {{\n\
                     {local}_items[{local}_index] = {convert}(rb_ary_entry({local}_array, {local}_index));\n\
                     }}"
                )),
            ],
            values,
            cleanup: vec![Statement::new(format!("RB_ALLOCV_END({local}_storage);"))],
            ..Self::default()
        })
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
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
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
            ReturnPlan::DirectViaReturnSlot {
                ty: DirectValueType::Record(record),
            } => Ok(Self::plain(ReturnKind::Direct {
                c_type: direct_record_type(*record, bridge)?,
                convert: RecordSymbols::new(*record, context)?.boxer().clone(),
            })),
            ReturnPlan::EncodedViaReturnSlot {
                codec,
                shape: native::BufferShape::Buffer,
                ..
            } => {
                Support::new(context).codec(codec)?;
                let body = Reader::new(context).decode(codec, "boltffi_value")?;
                Self::owned(decoder()?, body, abi_return)
            }
            ReturnPlan::ScalarOptionViaReturnSlot {
                primitive,
                enum_target: None,
            } => {
                let read = PrimitiveSymbols::new(*primitive).reader()?;
                let body = Statement::new(format!(
                    "if (boltffi_ruby_read_option_tag(reader)) {{\n\
                     boltffi_value = {read}(reader);\n\
                     }}"
                ));
                Self::owned(decoder()?, body, abi_return)
            }
            ReturnPlan::DirectVecViaReturnSlot { element } => {
                let (element_type, read) = match element {
                    DirectVectorElementType::Primitive(primitive) => {
                        let symbols = PrimitiveSymbols::new(primitive.primitive());
                        (symbols.c_type()?, symbols.element_reader()?)
                    }
                    DirectVectorElementType::Record(record) => (
                        direct_record_type(*record, bridge)?,
                        RecordSymbols::new(*record, context)?.reader().clone(),
                    ),
                    _ => return unsupported("direct vector element"),
                };
                let body = Statement::new(format!(
                    "long boltffi_count = boltffi_ruby_raw_count(reader, sizeof({element_type}));\n\
                     boltffi_value = rb_ary_new_capa(boltffi_count);\n\
                     for (long boltffi_index = 0; boltffi_index < boltffi_count; boltffi_index++) {{\n\
                     rb_ary_push(boltffi_value, {read}(reader));\n\
                     }}"
                ));
                Self::owned(decoder()?, body, abi_return)
            }
            ReturnPlan::DirectViaReturnSlot {
                ty: DirectValueType::Enum(_),
            }
            | ReturnPlan::ScalarOptionViaReturnSlot { .. } => unsupported("enum return"),
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

/// Returns the C typedef the bridge emits for a direct record.
pub fn direct_record_type(record: RecordId, bridge: &CBridgeContract) -> Result<TypeFragment> {
    bridge
        .source_direct_record(record)
        .map(|record| TypeFragment::new(record.name()))
        .ok_or(Error::BrokenBridgeContract {
            bridge: "c",
            invariant: "direct record has no C typedef",
        })
}
