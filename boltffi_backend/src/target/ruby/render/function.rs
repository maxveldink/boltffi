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
    bridge::c::{CBridgeContract, Parameter, Type},
    core::{Emitted, Error, RenderContext, Result},
    target::ruby::{
        codec::{read::Reader, write::Writer},
        name_style::Name,
        support::{Support, unsupported},
        symbol::{PrimitiveSymbols, RecordSymbols},
        syntax::Identifier,
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
    wrapper: String,
    arity: usize,
    setup: Vec<String>,
    call: String,
    cleanup: Vec<String>,
    result: String,
    helpers: Vec<String>,
    decoder: Option<Decoder>,
}

/// The `rb_define_module_function` call for one rendered function.
pub struct Registration {
    pub ruby_name: Identifier,
    pub wrapper: String,
    pub arity: i32,
}

/// A C function that decodes the buffer a function returns.
struct Decoder {
    name: String,
    body: String,
}

/// C code that moves one argument into native call slots.
#[derive(Default)]
struct Argument {
    setup: Vec<String>,
    values: Vec<String>,
    cleanup: Vec<String>,
    helpers: Vec<String>,
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
                let value = if variadic {
                    format!("argv[{index}]")
                } else {
                    format!("boltffi_arg_{index}")
                };
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
        let call_arguments = arguments
            .iter()
            .flat_map(|argument| argument.values.iter().cloned())
            .collect::<Vec<_>>()
            .join(", ");
        let native_call = format!("{}({call_arguments})", abi.name());
        let conversion = ReturnConversion::from_plan(
            callable.returns().plan(),
            &registration.wrapper,
            abi.returns(),
            bridge,
            context,
        )?;
        let (call, result) = conversion.call(&native_call)?;
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

    fn signature(&self) -> String {
        if self.variadic() {
            return "int argc, VALUE *argv, VALUE self".to_owned();
        }
        std::iter::once("VALUE self".to_owned())
            .chain((0..self.arity).map(|index| format!("VALUE boltffi_arg_{index}")))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl Registration {
    pub fn from_declaration(declaration: &FunctionDecl<Native>) -> Result<Self> {
        let arity = declaration.callable().params().len();
        Ok(Self {
            ruby_name: Name::new(declaration.name()).module_function()?,
            wrapper: format!("boltffi_ruby_fn_{}", declaration.symbol().name().as_str()),
            arity: if arity > MAX_FIXED_ARITY {
                -1
            } else {
                arity as i32
            },
        })
    }
}

/// Where one Ruby argument comes from inside the wrapper.
struct ArgumentSlot<'wrapper> {
    index: usize,
    value: String,
    wrapper: &'wrapper str,
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
                let mut writer = Writer::new(context, slot.value.as_str(), helper_prefix);
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
            setup: vec![format!(
                "{} {local} = boltffi_ruby_to_{}({});",
                symbols.c_type()?,
                symbols.stem()?,
                slot.value
            )],
            values: vec![local],
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
            setup: vec![format!(
                "{c_type} {local} = {}({});",
                symbols.unboxer(),
                slot.value
            )],
            values: vec![if by_pointer {
                format!("&{local}")
            } else {
                local
            }],
            ..Self::default()
        })
    }

    fn encoded(slot: &ArgumentSlot<'_>, statements: String, helpers: Vec<String>) -> Self {
        let writer = format!("boltffi_writer_{}", slot.index);
        Self {
            setup: vec![
                format!("boltffi_ruby_writer {writer};"),
                format!("boltffi_ruby_writer_init(&{writer});"),
                format!("{{\nboltffi_ruby_writer *writer = &{writer};\n{statements}\n}}"),
            ],
            values: vec![format!("{writer}.ptr"), format!("{writer}.len")],
            cleanup: vec![format!("RB_GC_GUARD({writer}.heap);")],
            helpers,
        }
    }

    fn scalar_option(slot: &ArgumentSlot<'_>, primitive: Primitive) -> Result<Self> {
        let stem = PrimitiveSymbols::new(primitive).stem()?;
        let writer = format!("boltffi_writer_{}", slot.index);
        let value = &slot.value;
        Ok(Self {
            setup: vec![
                format!("boltffi_ruby_writer {writer};"),
                format!("boltffi_ruby_writer_init(&{writer});"),
                format!(
                    "if (NIL_P({value})) {{\n\
                     boltffi_ruby_write_raw_u8(&{writer}, 0);\n\
                     }} else {{\n\
                     boltffi_ruby_write_raw_u8(&{writer}, 1);\n\
                     boltffi_ruby_write_{stem}(&{writer}, {value});\n\
                     }}"
                ),
            ],
            values: vec![format!("{writer}.ptr"), format!("{writer}.len")],
            cleanup: vec![format!("RB_GC_GUARD({writer}.heap);")],
            helpers: Vec::new(),
        })
    }

    fn direct_vector(
        slot: &ArgumentSlot<'_>,
        element: &DirectVectorElementType,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let index = slot.index;
        let (c_type, convert, values) = match element {
            DirectVectorElementType::Primitive(primitive) => {
                let symbols = PrimitiveSymbols::new(primitive.primitive());
                (
                    symbols.c_type()?.to_owned(),
                    format!("boltffi_ruby_to_{}", symbols.stem()?),
                    vec![
                        format!("boltffi_items_{index}"),
                        format!("(uintptr_t)boltffi_count_{index}"),
                    ],
                )
            }
            DirectVectorElementType::Record(record) => {
                let c_type = direct_record_type(*record, bridge)?;
                let values = vec![
                    format!("(const uint8_t *)boltffi_items_{index}"),
                    format!("(uintptr_t)boltffi_count_{index} * sizeof({c_type})"),
                ];
                (
                    c_type,
                    RecordSymbols::new(*record, context)?.unboxer(),
                    values,
                )
            }
            _ => return unsupported("direct vector element"),
        };
        Ok(Self {
            setup: vec![
                format!(
                    "VALUE boltffi_array_{index} = boltffi_ruby_expect_array({});",
                    slot.value
                ),
                format!("long boltffi_count_{index} = RARRAY_LEN(boltffi_array_{index});"),
                format!("VALUE boltffi_storage_{index} = 0;"),
                format!(
                    "{c_type} *boltffi_items_{index} = RB_ALLOCV_N({c_type}, boltffi_storage_{index}, boltffi_count_{index});"
                ),
                format!(
                    "for (long boltffi_index_{index} = 0; boltffi_index_{index} < boltffi_count_{index}; boltffi_index_{index}++) {{\n\
                     boltffi_items_{index}[boltffi_index_{index}] = {convert}(rb_ary_entry(boltffi_array_{index}, boltffi_index_{index}));\n\
                     }}"
                ),
            ],
            values,
            cleanup: vec![format!("RB_ALLOCV_END(boltffi_storage_{index});")],
            helpers: Vec::new(),
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
    Direct { c_type: String, convert: String },
    Owned { decoder: String },
}

impl ReturnConversion {
    fn from_plan(
        plan: &ReturnPlan<Native, OutOfRust>,
        wrapper: &str,
        abi_return: &Type,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let decoder_name = format!("{wrapper}_decode");
        match plan {
            ReturnPlan::Void => Ok(Self::plain(ReturnKind::Void)),
            ReturnPlan::DirectViaReturnSlot {
                ty: DirectValueType::Primitive(primitive),
            } => {
                let symbols = PrimitiveSymbols::new(*primitive);
                Ok(Self::plain(ReturnKind::Direct {
                    c_type: symbols.c_type()?.to_owned(),
                    convert: format!("boltffi_ruby_from_{}", symbols.stem()?),
                }))
            }
            ReturnPlan::DirectViaReturnSlot {
                ty: DirectValueType::Record(record),
            } => Ok(Self::plain(ReturnKind::Direct {
                c_type: direct_record_type(*record, bridge)?,
                convert: RecordSymbols::new(*record, context)?.boxer(),
            })),
            ReturnPlan::EncodedViaReturnSlot {
                codec,
                shape: native::BufferShape::Buffer,
                ..
            } => {
                Support::new(context).codec(codec)?;
                let body = Reader::new(context).decode(codec, "boltffi_value")?;
                Self::owned(decoder_name, body, abi_return)
            }
            ReturnPlan::ScalarOptionViaReturnSlot {
                primitive,
                enum_target: None,
            } => {
                let stem = PrimitiveSymbols::new(*primitive).stem()?;
                let body = format!(
                    "if (boltffi_ruby_read_option_tag(reader)) {{\n\
                     boltffi_value = boltffi_ruby_read_{stem}(reader);\n\
                     }}"
                );
                Self::owned(decoder_name, body, abi_return)
            }
            ReturnPlan::DirectVecViaReturnSlot { element } => {
                let (element_type, read) = match element {
                    DirectVectorElementType::Primitive(primitive) => {
                        let symbols = PrimitiveSymbols::new(primitive.primitive());
                        (
                            symbols.c_type()?.to_owned(),
                            format!("boltffi_ruby_read_{}", symbols.stem()?),
                        )
                    }
                    DirectVectorElementType::Record(record) => (
                        direct_record_type(*record, bridge)?,
                        RecordSymbols::new(*record, context)?.reader(),
                    ),
                    _ => return unsupported("direct vector element"),
                };
                let body = format!(
                    "long boltffi_count = boltffi_ruby_raw_count(reader, sizeof({element_type}));\n\
                     boltffi_value = rb_ary_new_capa(boltffi_count);\n\
                     for (long boltffi_index = 0; boltffi_index < boltffi_count; boltffi_index++) {{\n\
                     rb_ary_push(boltffi_value, {read}(reader));\n\
                     }}"
                );
                Self::owned(decoder_name, body, abi_return)
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

    fn owned(name: String, body: String, abi_return: &Type) -> Result<Self> {
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

    /// Returns the call statement and the Ruby result expression.
    fn call(&self, native_call: &str) -> Result<(String, String)> {
        Ok(match &self.kind {
            ReturnKind::Void => (format!("{native_call};"), "Qnil".to_owned()),
            ReturnKind::Direct { c_type, convert } => (
                format!("{c_type} boltffi_result = {native_call};"),
                format!("{convert}(boltffi_result)"),
            ),
            ReturnKind::Owned { decoder } => (
                format!("FfiBuf_u8 boltffi_result = {native_call};"),
                format!("boltffi_ruby_decode_owned(boltffi_result, {decoder})"),
            ),
        })
    }
}

/// Returns the C typedef the bridge emits for a direct record.
pub fn direct_record_type(record: RecordId, bridge: &CBridgeContract) -> Result<String> {
    bridge
        .source_direct_record(record)
        .map(|record| record.name().to_owned())
        .ok_or(Error::BrokenBridgeContract {
            bridge: "c",
            invariant: "direct record has no C typedef",
        })
}
