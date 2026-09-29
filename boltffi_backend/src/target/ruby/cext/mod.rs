//! Ruby C extension host (experimental).
//!
//! The Ruby target renders a C extension that links the Rust static library
//! and calls the shared C ABI (`CBridge`) directly. Every value will cross as
//! a plain Ruby object that the extension builds eagerly, so Ruby code, and
//! YJIT, see ordinary Ruby values.
//!
//! This scaffold declares the host, its syntax, and its naming rules. The
//! plumbing and the renderer follow in later changes.

mod support;

use boltffi_binding::{
    Bindings, CallbackDecl, ClassDecl, ConstantDecl, CustomTypeDecl, EnumDecl, FunctionDecl,
    Native, RecordDecl, StreamDecl,
};

use crate::{
    bridge::c::{CBridge, CBridgeContract},
    core::{
        BindingCapability, BridgeCapability, CapabilityRequirements, Emitted, GeneratedOutput,
        HostCapabilities, RenderContext, RenderedDeclaration, Result, Target, contract::sealed,
        host,
    },
};

use self::support::unsupported;
use super::name_style::extension_stem;

/// Ruby C extension host, paired with the shared C ABI bridge.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub struct RubyCExtHost;

impl RubyCExtHost {
    /// Creates a Ruby host renderer.
    pub fn new() -> Self {
        Self
    }

    /// Creates the backend target stack for this Ruby host.
    ///
    /// Ruby calls the C ABI directly, so the stack is `CBridge` alone. The
    /// bridge writes its header beside the extension source.
    pub fn into_target(self, bindings: &Bindings<Native>) -> Result<Target<Self, CBridge>> {
        let stem = extension_stem(&bindings.package().name().as_path_string());
        let header = format!("ext/{stem}/boltffi.h");
        Ok(Target::new(self, CBridge::new(header)?))
    }
}

impl host::HostBackend for RubyCExtHost {
    type Surface = Native;
    type Bridge = CBridgeContract;
    type Syntax = super::syntax::Syntax;

    fn name(&self) -> &'static str {
        "ruby"
    }

    fn binding_capabilities(&self) -> HostCapabilities {
        HostCapabilities::new()
            .unsupported(
                BindingCapability::Records,
                "records are not implemented in the Ruby host",
            )
            .unsupported(
                BindingCapability::Functions,
                "functions are not implemented in the Ruby host",
            )
            .unsupported(
                BindingCapability::Enums,
                "enums are not implemented in the Ruby host",
            )
            .unsupported(
                BindingCapability::Classes,
                "classes are not implemented in the Ruby host",
            )
            .unsupported(
                BindingCapability::Callbacks,
                "callbacks are not implemented in the Ruby host",
            )
            .unsupported(
                BindingCapability::Streams,
                "streams are not implemented in the Ruby host",
            )
            .unsupported(
                BindingCapability::Constants,
                "constants are not implemented in the Ruby host",
            )
            .unsupported(
                BindingCapability::CustomTypes,
                "custom types are not implemented in the Ruby host",
            )
    }

    fn bridge_capabilities(&self) -> CapabilityRequirements<BridgeCapability> {
        CapabilityRequirements::new().require(BridgeCapability::CAbi)
    }

    fn record(
        &self,
        _decl: &RecordDecl<Self::Surface>,
        _bridge: &Self::Bridge,
        _context: &RenderContext<Self::Surface>,
    ) -> Result<Emitted> {
        unsupported("record")
    }

    fn enumeration(
        &self,
        _decl: &EnumDecl<Self::Surface>,
        _bridge: &Self::Bridge,
        _context: &RenderContext<Self::Surface>,
    ) -> Result<Emitted> {
        unsupported("enum")
    }

    fn function(
        &self,
        _decl: &FunctionDecl<Self::Surface>,
        _bridge: &Self::Bridge,
        _context: &RenderContext<Self::Surface>,
    ) -> Result<Emitted> {
        unsupported("function")
    }

    fn class(
        &self,
        _decl: &ClassDecl<Self::Surface>,
        _bridge: &Self::Bridge,
        _context: &RenderContext<Self::Surface>,
    ) -> Result<Emitted> {
        unsupported("class")
    }

    fn callback(
        &self,
        _decl: &CallbackDecl<Self::Surface>,
        _bridge: &Self::Bridge,
        _context: &RenderContext<Self::Surface>,
    ) -> Result<Emitted> {
        unsupported("callback")
    }

    fn stream(
        &self,
        _decl: &StreamDecl<Self::Surface>,
        _bridge: &Self::Bridge,
        _context: &RenderContext<Self::Surface>,
    ) -> Result<Emitted> {
        unsupported("stream")
    }

    fn constant(
        &self,
        _decl: &ConstantDecl<Self::Surface>,
        _bridge: &Self::Bridge,
        _context: &RenderContext<Self::Surface>,
    ) -> Result<Emitted> {
        unsupported("constant")
    }

    fn custom_type(
        &self,
        _decl: &CustomTypeDecl,
        _bridge: &Self::Bridge,
        _context: &RenderContext<Self::Surface>,
    ) -> Result<Emitted> {
        unsupported("custom type")
    }

    fn assemble<'decl>(
        &self,
        _bindings: &Bindings<Self::Surface>,
        _bridge: &Self::Bridge,
        _context: &RenderContext<Self::Surface>,
        _declarations: Vec<RenderedDeclaration<'decl, Self::Surface>>,
    ) -> Result<GeneratedOutput> {
        // Every declaration is unsupported until the renderer lands, so there
        // is nothing to assemble yet.
        Ok(GeneratedOutput::new(Vec::new(), Vec::new()))
    }
}

impl sealed::HostBackend for RubyCExtHost {}
