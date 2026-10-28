//! `boltffi pack ruby`: generated gem sources plus the host static library.
//!
//! The pack generates the sources and builds the Rust static library from
//! one binding expansion, copies the library into the generated extension
//! directory, and writes its link metadata beside it. `gem build` in that
//! tree then produces an installable gem.
//!
//! Each generation deletes the earlier gem tree first, so a library in the
//! tree always comes from the pack that wrote the current sources.

use std::path::{Path, PathBuf};

use boltffi_backend::target::ruby::name_style::extension_stem;
use boltffi_bindgen::target::Target;

use crate::{
    build::{
        BindingExpansion, BuildOptions, BuildSelection, Builder, CargoBuildProfile, OutputCallback,
        native_link::NativeLinkMetadata, resolve_build_profile,
    },
    cargo::Cargo,
    cli::{CliError, Result},
    commands::{generate::bindings::render_ruby, pack::PackRubyOptions},
    config::{Config, TargetSection},
    pack::{print_cargo_line, resolve_build_cargo_args},
    reporter::Reporter,
    target::NativeHostPlatform,
};

pub(crate) fn pack_ruby(
    config: &Config,
    options: PackRubyOptions,
    reporter: &Reporter,
) -> Result<()> {
    if !config.is_ruby_enabled() {
        return Err(failed("targets.ruby.enabled = false"));
    }
    if !config.should_process(Target::Ruby, options.experimental) {
        return Err(failed(
            "ruby is experimental, use --experimental or add \"ruby\" to experimental",
        ));
    }
    let platform = supported_host(NativeHostPlatform::current())?;

    reporter.section("💎", "Packing Ruby");
    let extension_dir = config
        .ruby_output()
        .join("ext")
        .join(extension_stem(config.ruby_gem_name()));
    if options.execution.no_build {
        ensure_packed(&extension_dir)?;
        reporter.finish();
        return Ok(());
    }
    if !options.execution.regenerate {
        return Err(failed(
            "pack ruby builds the static library and the sources together; remove '--regenerate false'",
        ));
    }

    let build_cargo_args =
        resolve_build_cargo_args(config, TargetSection::Ruby, &options.execution.cargo_args);
    let cargo = Cargo::current(&build_cargo_args)?;
    if let Some(target) = cargo
        .target_selector()
        .map(str::to_owned)
        .or_else(|| cargo.configured_build_target())
    {
        return Err(failed(&format!(
            "pack ruby is host-only; remove cargo target '{target}'"
        )));
    }
    let build_profile = resolve_build_profile(options.execution.release, &build_cargo_args);
    let binding_expansion = BindingExpansion::resolve(config, &build_cargo_args)?;
    if !binding_expansion.selected_library().builds_staticlib() {
        return Err(failed(
            "pack ruby requires the selected Rust library target to build a staticlib",
        ));
    }

    // The sources and the static library come from one expansion, so
    // `[cargo.command_args].generate` cannot add a feature the library lacks.
    let step = reporter.step("Generating Ruby bindings");
    render_ruby(
        config,
        &binding_expansion,
        &config.ruby_output(),
        options.execution.deny_skipped,
    )?;
    step.finish_success();

    let static_library = platform.static_library_filename(binding_expansion.artifact_name());
    let built = binding_expansion
        .target_directory()
        .join(build_profile.output_directory_name())
        .join(&static_library);
    let vendored = extension_dir.join(&static_library);

    let step = reporter.step("Building Rust static library");
    let on_output: Option<OutputCallback> = step
        .is_verbose()
        .then(|| Box::new(print_cargo_line) as OutputCallback);
    let native_link = Builder::new(
        config,
        BuildOptions {
            release: matches!(build_profile, CargoBuildProfile::Release),
            selection: BuildSelection::Expanded(Box::new(binding_expansion)),
            on_output,
            extra_env: Vec::new(),
        },
    )
    .build_host_with_native_link_metadata()?;
    step.finish_success();

    let step = reporter.step("Vendoring static library into the gem");
    std::fs::copy(&built, &vendored).map_err(|source| CliError::CopyFailed {
        from: built,
        to: vendored.clone(),
        source,
    })?;
    native_link.write(&link_metadata_path(&vendored))?;
    step.finish_success();
    reporter.finish();
    Ok(())
}

/// Fails before `pack all` packs any target when `pack ruby` would fail on
/// this host.
pub(crate) fn ensure_ruby_host_supported(config: &Config, experimental: bool) -> Result<()> {
    if config.should_process(Target::Ruby, experimental) {
        supported_host(NativeHostPlatform::current())?;
    }
    Ok(())
}

/// Returns the host when `pack ruby` can build on it.
///
/// `extconf.rb` links a Unix `lib<name>.a` archive, so the pack refuses a
/// Windows host before it generates or builds anything.
fn supported_host(host: Option<NativeHostPlatform>) -> Result<NativeHostPlatform> {
    match host {
        Some(NativeHostPlatform::WindowsX86_64 | NativeHostPlatform::WindowsAarch64) | None => {
            Err(failed("pack ruby supports only macOS and Linux hosts"))
        }
        Some(host) => Ok(host),
    }
}

/// Checks that an earlier pack left a library and its link metadata in the
/// extension directory. A later generation deletes both.
fn ensure_packed(extension_dir: &Path) -> Result<()> {
    let packed = std::fs::read_dir(extension_dir)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .find(|path| path.extension().is_some_and(|extension| extension == "a"));
    match packed {
        Some(library) => NativeLinkMetadata::read(&link_metadata_path(&library)).map(drop),
        None => Err(failed(
            "no packed Ruby static library; run pack ruby without --no-build first",
        )),
    }
}

fn link_metadata_path(library: &Path) -> PathBuf {
    let mut path = library.as_os_str().to_owned();
    path.push(".boltffi-link.json");
    PathBuf::from(path)
}

fn failed(command: &str) -> CliError {
    CliError::CommandFailed {
        command: command.to_owned(),
        status: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_ruby_refuses_hosts_that_extconf_cannot_link() {
        use NativeHostPlatform::*;

        for host in [Some(WindowsX86_64), Some(WindowsAarch64), None] {
            assert!(supported_host(host).is_err(), "{host:?}");
        }
        for host in [DarwinArm64, DarwinX86_64, LinuxX86_64, LinuxAarch64] {
            assert_eq!(supported_host(Some(host)).ok(), Some(host));
        }
    }

    #[test]
    fn no_build_requires_a_library_and_its_link_metadata() {
        let extension_dir = tempfile::tempdir().unwrap();
        assert!(ensure_packed(&extension_dir.path().join("missing")).is_err());
        assert!(ensure_packed(extension_dir.path()).is_err());

        let library = extension_dir.path().join("libdemo.a");
        std::fs::write(&library, b"").unwrap();
        assert!(ensure_packed(extension_dir.path()).is_err());

        NativeLinkMetadata {
            native_static_libraries: vec!["-lc".to_owned()],
            native_link_search_paths: Vec::new(),
        }
        .write(&link_metadata_path(&library))
        .unwrap();
        ensure_packed(extension_dir.path()).unwrap();
    }
}
