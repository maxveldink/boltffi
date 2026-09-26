//! `boltffi pack ruby`: generated gem sources plus the host static library.
//!
//! The pack builds the Rust static library through the binding expansion,
//! copies it into the generated extension directory, and writes its link
//! metadata beside it. `gem build` or `ruby extconf.rb && make` in that tree
//! then needs no cargo.

use std::path::{Path, PathBuf};

use boltffi_bindgen::target::Target;

use crate::{
    build::{
        BindingExpansion, BuildOptions, BuildSelection, Builder, CargoBuildProfile, OutputCallback,
        resolve_build_profile,
    },
    cargo::Cargo,
    cli::{CliError, Result},
    commands::{
        generate::{GenerateOptions, GenerateTarget, run_generate_with_output},
        pack::PackRubyOptions,
    },
    config::Config,
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

    reporter.section("💎", "Packing Ruby");

    let build_cargo_args = resolve_build_cargo_args(config, &options.execution.cargo_args);
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

    if options.execution.regenerate && !options.execution.no_build {
        let step = reporter.step("Generating Ruby bindings");
        run_generate_with_output(
            config,
            GenerateOptions {
                target: GenerateTarget::Ruby,
                output: Some(config.ruby_output()),
                experimental: options.experimental,
                cargo_args: build_cargo_args.clone(),
                deny_skipped: options.execution.deny_skipped,
            },
        )?;
        step.finish_success();
    }

    let platform = NativeHostPlatform::current()
        .ok_or_else(|| failed("pack ruby is unsupported on this host platform"))?;
    let artifact_name = binding_expansion.artifact_name().to_owned();
    let static_library = platform.static_library_filename(&artifact_name);
    let profile_dir = binding_expansion
        .target_directory()
        .join(build_profile.output_directory_name());
    let extension_dir = extension_dir(&config.ruby_output())?;
    let vendored = extension_dir.join(&static_library);
    let link_metadata = extension_dir.join(format!("{static_library}.boltffi-link.json"));

    if options.execution.no_build {
        if !vendored.exists() || !link_metadata.exists() {
            return Err(failed(
                "no packed Ruby static library; run pack ruby without --no-build first",
            ));
        }
        reporter.finish();
        return Ok(());
    }

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
    let built = profile_dir.join(&static_library);
    std::fs::copy(&built, &vendored).map_err(|source| CliError::CopyFailed {
        from: built,
        to: vendored.clone(),
        source,
    })?;
    native_link.write(&link_metadata)?;
    step.finish_success();
    reporter.finish();
    Ok(())
}

/// Returns the one `ext/<stem>` directory that holds a generated `extconf.rb`.
fn extension_dir(output: &Path) -> Result<PathBuf> {
    let ext = output.join("ext");
    let mut candidates = std::fs::read_dir(&ext)
        .map_err(|source| CliError::ReadFailed {
            path: ext.clone(),
            source,
        })?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.join("extconf.rb").is_file())
        .collect::<Vec<_>>();
    match candidates.len() {
        1 => Ok(candidates.remove(0)),
        0 => Err(failed(&format!(
            "no generated Ruby extension under {}; run generate ruby first",
            ext.display()
        ))),
        _ => Err(failed(&format!(
            "more than one generated Ruby extension under {}",
            ext.display()
        ))),
    }
}

fn failed(command: &str) -> CliError {
    CliError::CommandFailed {
        command: command.to_owned(),
        status: None,
    }
}
