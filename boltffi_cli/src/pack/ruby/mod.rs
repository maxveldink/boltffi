//! `boltffi pack ruby`: generated gem sources plus the Rust crate source.
//!
//! The pack generates the gem sources and copies the crate folder into the
//! extension directory. The generated `extconf.rb` builds that crate with
//! cargo when the gem installs, so `gem build` in the output directory makes
//! one gem for every platform.

mod crate_source;

use std::path::{Path, PathBuf};

use boltffi_backend::target::ruby::name_style::extension_stem;
use boltffi_bindgen::target::Target;

use crate::{
    build::BindingExpansion,
    cli::{CliError, Result},
    commands::{generate::bindings::render_ruby, pack::PackRubyOptions},
    config::{Config, TargetSection},
    pack::resolve_build_cargo_args,
    reporter::Reporter,
};

/// The folder in `ext/<stem>/` that receives the crate source.
const CRATE_DIRECTORY: &str = "crate";

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
    if !options.execution.regenerate {
        return Err(failed(
            "pack ruby generates the sources from the crate that it copies; remove '--regenerate false'",
        ));
    }

    let build_cargo_args =
        resolve_build_cargo_args(config, TargetSection::Ruby, &options.execution.cargo_args);
    let binding_expansion = BindingExpansion::resolve(config, &build_cargo_args)?;
    if !binding_expansion.selected_library().builds_staticlib() {
        return Err(failed(
            "pack ruby requires the selected Rust library target to build a staticlib",
        ));
    }

    // The sources and the features in `extconf.rb` come from one expansion,
    // so `[cargo.command_args].generate` cannot add a feature the build lacks.
    let output = config.ruby_output();
    let step = reporter.step("Generating Ruby bindings");
    render_ruby(
        config,
        &binding_expansion,
        &output,
        options.execution.deny_skipped,
    )?;
    step.finish_success();

    let step = reporter.step("Copying the crate source into the gem");
    let manifest_path = binding_expansion.manifest_path();
    let crate_root = manifest_path.parent().ok_or_else(|| {
        failed(&format!(
            "manifest path '{}' has no parent directory",
            manifest_path.display()
        ))
    })?;
    let destination = extension_dir(&output, config.ruby_gem_name())?.join(CRATE_DIRECTORY);
    let external = crate_source::copy(
        crate_root,
        &destination,
        &[binding_expansion.target_directory(), &output],
    )?;
    step.finish_success();
    external.iter().for_each(|dependency| {
        reporter.warning(&format!(
            "{}: path dependency `{}` is outside the crate folder. The gem builds only where {} exists.",
            dependency.manifest.display(),
            dependency.name,
            dependency.path.display()
        ));
    });
    reporter.finish();
    Ok(())
}

/// Returns the generated extension directory for the current gem.
fn extension_dir(output: &Path, gem: &str) -> Result<PathBuf> {
    let directory = output.join("ext").join(extension_stem(gem));
    if !directory.join("extconf.rb").is_file() {
        return Err(failed(&format!(
            "no generated Ruby extension at {}; run generate ruby first",
            directory.display()
        )));
    }
    Ok(directory)
}

fn failed(command: &str) -> CliError {
    CliError::CommandFailed {
        command: command.to_owned(),
        status: None,
    }
}
