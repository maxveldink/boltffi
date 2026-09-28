//! Ruby host renderer (experimental).
//!
//! The Ruby target renders a C extension that links the Rust static library
//! and calls the shared C ABI (`CBridge`) directly. Every value crosses as a
//! plain Ruby object: `Integer`, `Float`, `true`/`false`, `String`, `Array`,
//! `Hash`, `nil`, and one frozen `Data` class per record. The extension builds
//! those objects eagerly, so Ruby code, and YJIT, see ordinary Ruby values.
//!
//! The target renders synchronous free functions and records. Enums, classes,
//! callbacks, streams, async functions, constants, custom types, and fallible
//! functions are not supported yet.

mod codec;
mod name_style;
mod render;
mod support;
mod symbol;
/// Ruby syntax fragments.
pub mod syntax;

use boltffi_binding::{
    Bindings, CallbackDecl, ClassDecl, ConstantDecl, CustomTypeDecl, EnumDecl, FunctionDecl,
    Native, RecordDecl, StreamDecl,
};

use crate::{
    bridge::c::{CBridge, CBridgeContract},
    core::{
        BindingCapability, BridgeCapability, CapabilityRequirements, Emitted, FilePath,
        GeneratedFile, GeneratedOutput, HostCapabilities, RenderContext, RenderedDeclaration,
        Result, Target, contract::sealed, host,
    },
};

use self::{
    name_style::{default_module, extension_stem, package_snake},
    render::{extension, function::Function, package::Package, record::Record},
    support::unsupported,
    syntax::ConstantPath,
};

/// Ruby host renderer paired with the shared C ABI bridge.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub struct RubyHost {
    module: Option<ConstantPath>,
    gem: Option<String>,
    version: Option<String>,
    library: Option<String>,
    cargo_manifest: Option<String>,
    active_features: String,
    feature_args: Vec<String>,
}

impl RubyHost {
    /// Creates a Ruby host renderer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Selects the Ruby module that holds the functions and records, such as
    /// `MyLib::Native`.
    pub fn module_name(mut self, module: &str) -> Result<Self> {
        self.module = Some(ConstantPath::parse(module)?);
        Ok(self)
    }

    /// Selects the gem name. The extension stem is the gem name in
    /// `snake_case`.
    pub fn gem_name(mut self, gem: impl Into<String>) -> Self {
        self.gem = Some(gem.into());
        self
    }

    /// Selects the gem version.
    pub fn version(mut self, version: Option<String>) -> Self {
        self.version = version;
        self
    }

    /// Selects the Rust library artifact name, `demo` for `libdemo.a`.
    pub fn native_library(mut self, library: impl Into<String>) -> Self {
        self.library = Some(library.into());
        self
    }

    /// Records the crate manifest path, relative to the extension directory,
    /// that the generated `extconf.rb` builds when no prebuilt library exists.
    pub fn cargo_manifest(mut self, manifest: impl Into<String>) -> Self {
        self.cargo_manifest = Some(manifest.into());
        self
    }

    /// Records the cargo feature selection of the binding expansion: the
    /// active features, comma separated, and the cargo arguments that
    /// selected them. The generated `extconf.rb` replays the arguments, so
    /// the library exports every function the extension calls.
    pub fn cargo_features(mut self, active: impl Into<String>, arguments: Vec<String>) -> Self {
        self.active_features = active.into();
        self.feature_args = arguments;
        self
    }

    /// Creates the backend target stack for this Ruby host.
    ///
    /// Ruby calls the C ABI directly, so the stack is `CBridge` alone. The
    /// bridge writes its header beside the extension source.
    pub fn into_target(self, bindings: &Bindings<Native>) -> Result<Target<Self, CBridge>> {
        let header = format!("ext/{}/boltffi.h", self.stem(bindings));
        Ok(Target::new(self, CBridge::new(header)?))
    }

    /// The gem name: the configured name, or the Cargo package name.
    fn gem(&self, bindings: &Bindings<Native>) -> String {
        self.gem
            .clone()
            .unwrap_or_else(|| bindings.package().name().as_path_string())
    }

    fn stem(&self, bindings: &Bindings<Native>) -> String {
        extension_stem(&self.gem(bindings))
    }

    fn module(&self, bindings: &Bindings<Native>) -> Result<ConstantPath> {
        self.module
            .clone()
            .map(Ok)
            .unwrap_or_else(|| default_module(bindings.package().name()))
    }
}

impl host::HostBackend for RubyHost {
    type Surface = Native;
    type Bridge = CBridgeContract;
    type Syntax = syntax::Syntax;

    fn name(&self) -> &'static str {
        "ruby"
    }

    fn binding_capabilities(&self) -> HostCapabilities {
        HostCapabilities::new()
            .stable(BindingCapability::Records)
            .stable(BindingCapability::Functions)
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
        decl: &RecordDecl<Self::Surface>,
        bridge: &Self::Bridge,
        context: &RenderContext<Self::Surface>,
    ) -> Result<Emitted> {
        Record::from_declaration(decl, bridge, context)?.render()
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
        decl: &FunctionDecl<Self::Surface>,
        bridge: &Self::Bridge,
        context: &RenderContext<Self::Surface>,
    ) -> Result<Emitted> {
        Function::from_declaration(decl, bridge, context)?.render()
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
        bindings: &Bindings<Self::Surface>,
        _bridge: &Self::Bridge,
        _context: &RenderContext<Self::Surface>,
        declarations: Vec<RenderedDeclaration<'decl, Self::Surface>>,
    ) -> Result<GeneratedOutput> {
        let gem = self.gem(bindings);
        let stem = self.stem(bindings);
        let module = self.module(bindings)?;
        let artifact = self
            .library
            .clone()
            .unwrap_or_else(|| package_snake(bindings.package().name()));
        let version = self
            .version
            .clone()
            .or_else(|| bindings.package().version().map(str::to_owned))
            .unwrap_or_else(|| "0.1.0".to_owned());
        let crate_name = bindings.package().name().as_path_string();
        let package = Package {
            gem: &gem,
            version: &version,
            stem: &stem,
            module: &module,
            artifact: &artifact,
            cargo_manifest: self.cargo_manifest.as_deref(),
            active_features: &self.active_features,
            feature_args: &self.feature_args,
            crate_name: &crate_name,
        };
        let source = GeneratedFile::new(
            FilePath::new(format!("{}/{stem}.c", package.extension_dir()))?,
            extension::render(&stem, &module, declarations)?,
        );
        let files = std::iter::once(source).chain(package.render()?).collect();
        Ok(GeneratedOutput::new(files, Vec::new()))
    }
}

impl sealed::HostBackend for RubyHost {}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use boltffi_ast::PackageInfo;
    use boltffi_binding::{Bindings, Native, lower};

    use crate::core::{Error, GeneratedOutput};

    use super::RubyHost;

    fn bindings(source: &str) -> Bindings<Native> {
        let source = boltffi_scan::scan_file(
            syn::parse_str(source).expect("valid source"),
            PackageInfo::new("demo", Some("1.2.3".to_owned())),
        )
        .expect("source should scan");
        lower::<Native>(&source).expect("source should lower")
    }

    fn render(host: RubyHost, source: &str) -> GeneratedOutput {
        let bindings = bindings(source);
        host.into_target(&bindings)
            .expect("Ruby target")
            .render(&bindings)
            .expect("Ruby target should render")
    }

    fn file<'output>(output: &'output GeneratedOutput, path: &str) -> &'output str {
        output
            .files()
            .iter()
            .find(|file| file.path().as_path() == Path::new(path))
            .map(|file| file.contents())
            .unwrap_or_else(|| panic!("generated file {path}"))
    }

    #[test]
    fn ruby_extension_for_scalar_string_and_collection_functions() {
        let output = render(
            RubyHost::new(),
            r#"
            use std::collections::HashMap;

            #[export]
            pub fn add(left: i32, right: i32) -> i32 { left + right }

            #[export]
            pub fn greet(name: String) -> String { format!("hi {name}") }

            #[export]
            pub fn total(values: Vec<u64>) -> u64 { values.iter().sum() }

            #[export]
            pub fn tags(names: Vec<String>) -> HashMap<String, Vec<i32>> { HashMap::new() }

            #[export]
            pub fn half(value: Option<i32>) -> Option<f64> { value.map(|value| value as f64 / 2.0) }

            #[export]
            pub fn noop() {}
            "#,
        );

        insta::assert_snapshot!("ruby_functions", file(&output, "ext/demo/demo.c"));
    }

    #[test]
    fn ruby_extension_for_direct_and_encoded_records() {
        let output = render(
            RubyHost::new(),
            r#"
            use std::collections::HashMap;

            #[data]
            pub struct Point { pub x: f64, pub y: f64 }

            #[data]
            pub struct Shape { pub name: String, pub hash: u32, pub points: Vec<Point>, pub center: Option<Point>, pub labels: HashMap<String, String> }

            #[export]
            pub fn echo_point(point: Point) -> Point { point }

            #[export]
            pub fn points(count: u32) -> Vec<Point> { Vec::new() }

            #[export]
            pub fn echo_shape(shape: Shape) -> Shape { shape }
            "#,
        );

        insta::assert_snapshot!("ruby_records", file(&output, "ext/demo/demo.c"));
    }

    #[test]
    fn ruby_package_files_use_the_configured_gem_module_and_manifest() {
        let host = RubyHost::new()
            .gem_name("my-lib")
            .module_name("MyLib::Native")
            .expect("valid module")
            .native_library("my_lib")
            .cargo_manifest("../../Cargo.toml")
            .cargo_features(
                "default,ffi",
                vec!["--features".to_owned(), "my-lib/ffi".to_owned()],
            );
        let output = render(host, "#[export] pub fn ping() -> bool { true }");

        let paths = output
            .files()
            .iter()
            .map(|file| file.path().as_path().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            paths,
            [
                "ext/my_lib/boltffi.h",
                "ext/my_lib/my_lib.c",
                "lib/my_lib.rb",
                "ext/my_lib/extconf.rb",
                "ext/my_lib/boltffi_ruby.h",
                "my-lib.gemspec",
            ]
        );
        insta::assert_snapshot!(
            "ruby_package_files",
            ["lib/my_lib.rb", "ext/my_lib/extconf.rb", "my-lib.gemspec",]
                .map(|path| format!("==> {path}\n{}", file(&output, path)))
                .join("\n")
        );
        assert!(file(&output, "ext/my_lib/my_lib.c").contains(
            "RUBY_FUNC_EXPORTED void Init_my_lib(void) {\n    VALUE boltffi_module = rb_define_module(\"MyLib\");\n    boltffi_module = rb_define_module_under(boltffi_module, \"Native\");"
        ));
    }

    #[test]
    fn ruby_partial_render_skips_unsupported_declarations_with_coverage_entries() {
        let source = r#"
            #[data]
            pub enum Mode { Fast, Slow }

            #[export]
            pub fn current_mode() -> Mode { Mode::Fast }

            #[export]
            pub fn parse(text: String) -> Result<i32, String> { text.parse().map_err(|_| text) }

            #[export]
            pub fn ok() -> bool { true }

            #[export]
            pub fn flatten(value: Option<Option<i32>>) -> Option<i32> { value.flatten() }

            #[export]
            pub fn keep(values: Option<Vec<Option<i32>>>) -> Option<Vec<Option<i32>>> { values }
        "#;
        let bindings = bindings(source);
        let output = RubyHost::new()
            .into_target(&bindings)
            .expect("Ruby target")
            .render_partial(&bindings)
            .expect("partial render");

        let reasons = output
            .coverage()
            .unsupported()
            .iter()
            .map(|entry| format!("{}: {}", entry.declaration().name(), entry.reason()))
            .collect::<Vec<_>>();
        assert_eq!(
            reasons,
            [
                "mode: enums are not implemented in the Ruby host",
                "current::mode: enum return",
                "parse: fallible function",
                "flatten: nested optional",
            ]
        );
        let extension = file(&output, "ext/demo/demo.c");
        assert!(extension.contains("\"ok\""));
        // An optional inside a vector inside an optional keeps `None` and
        // `Some(None)` apart, so it is supported.
        assert!(extension.contains("\"keep\""));
        assert!(!extension.contains("\"parse\""));
        assert!(!extension.contains("\"current_mode\""));
    }

    #[test]
    fn ruby_complete_render_rejects_unsupported_declarations() {
        let bindings = bindings(
            r#"
            #[export]
            pub fn parse(text: String) -> Result<i32, String> { text.parse().map_err(|_| text) }
            "#,
        );
        let error = RubyHost::new()
            .into_target(&bindings)
            .expect("Ruby target")
            .render(&bindings)
            .expect_err("fallible functions are unsupported");

        assert!(matches!(
            error,
            Error::UnsupportedTarget {
                target: "ruby",
                shape: "fallible function"
            }
        ));
    }

    #[test]
    fn ruby_escaped_function_names_cannot_collide() {
        let bindings = bindings(
            r#"
            #[export]
            pub fn freeze() {}

            #[export]
            pub fn freeze_() {}
            "#,
        );
        let error = RubyHost::new()
            .into_target(&bindings)
            .expect("Ruby target")
            .render(&bindings)
            .expect_err("both functions want freeze_");

        assert!(matches!(
            error,
            Error::RubyNameCollision { ref name, .. } if name == "freeze_"
        ));
    }

    #[test]
    fn ruby_record_members_that_escape_to_one_name_collide() {
        let bindings = bindings(
            r#"
            #[data]
            pub struct Tagged { pub hash: String, pub hash_: String }

            #[export]
            pub fn tagged(value: Tagged) -> Tagged { value }
            "#,
        );
        let error = RubyHost::new()
            .into_target(&bindings)
            .expect("Ruby target")
            .render(&bindings)
            .expect_err("both fields want the member hash_");

        assert_eq!(
            error.to_string(),
            "ruby name collision in record `Tagged` members: `hash_` is used by field `hash` and field `hash_`"
        );
    }

    #[test]
    fn ruby_functions_with_more_than_fifteen_arguments_use_variadic_arity() {
        let params = (0..16)
            .map(|index| format!("a{index}: u8"))
            .collect::<Vec<_>>()
            .join(", ");
        let output = render(
            RubyHost::new(),
            &format!("#[export] pub fn wide({params}) -> u8 {{ a15 }}"),
        );
        let extension = file(&output, "ext/demo/demo.c");

        assert!(
            extension.contains(
                "(int argc, VALUE *argv, VALUE self) {\n    rb_check_arity(argc, 16, 16);"
            )
        );
        assert!(extension.contains("uint8_t boltffi_value_15 = boltffi_ruby_to_u8(argv[15]);"));
        assert!(extension.contains("\"wide\", boltffi_ruby_fn_boltffi_function_demo_wide, -1);"));
    }
}
