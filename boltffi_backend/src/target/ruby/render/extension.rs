//! The extension source file: forward declarations, definitions, and `Init_`.

use std::collections::BTreeSet;

use askama::Template;
use boltffi_binding::{DeclarationRef, Native};

use crate::core::{AuxChunk, Error, RenderedDeclaration, Result};
use crate::target::ruby::syntax::{Constant, ConstantPath};

use super::{function, record};

#[derive(Template)]
#[template(path = "target/ruby/extension.c", escape = "none")]
struct ExtensionTemplate<'extension> {
    stem: &'extension str,
    outer: &'extension Constant,
    nested: &'extension [Constant],
    forward_declarations: Vec<String>,
    definitions: Vec<String>,
    records: Vec<record::Registration>,
    functions: Vec<function::Registration>,
}

/// Renders the C source of the extension.
pub fn render(
    stem: &str,
    module: &ConstantPath,
    declarations: Vec<RenderedDeclaration<'_, Native>>,
) -> Result<String> {
    let (outer, nested) = module
        .segments()
        .split_first()
        .ok_or(Error::InvalidRubyIdentifier {
            identifier: module.to_string(),
        })?;
    let mut forward_declarations = Vec::new();
    let mut definitions = Vec::new();
    let mut records = Vec::new();
    let mut functions = Vec::new();
    for rendered in declarations {
        match rendered.declaration() {
            DeclarationRef::Record(declaration) => {
                records.push(record::Registration::from_declaration(declaration)?);
            }
            DeclarationRef::Function(declaration) => {
                functions.push(function::Registration::from_declaration(declaration)?);
            }
            _ => {}
        }
        let (primary, aux, _) = rendered.into_parts().1.into_parts();
        forward_declarations.extend(aux.into_iter().filter_map(|chunk| match chunk {
            AuxChunk::ForwardDecl(text) => Some(text.into_string()),
            _ => None,
        }));
        if !primary.is_empty() {
            definitions.push(primary.into_string());
        }
    }
    unique_names(
        module,
        "record constants",
        records.iter().map(|record| record.constant.as_str()),
    )?;
    unique_names(
        module,
        "module functions",
        functions.iter().map(|function| function.ruby_name.as_str()),
    )?;
    let source = ExtensionTemplate {
        stem,
        outer,
        nested,
        forward_declarations,
        definitions,
        records,
        functions,
    }
    .render()?;
    Ok(indent(&source))
}

fn unique_names<'name>(
    module: &ConstantPath,
    kind: &str,
    names: impl IntoIterator<Item = &'name str>,
) -> Result<()> {
    let mut seen = BTreeSet::new();
    names.into_iter().try_for_each(|name| {
        if seen.insert(name) {
            Ok(())
        } else {
            Err(Error::RubyNameCollision {
                scope: format!("{module} {kind}"),
                name: name.to_owned(),
            })
        }
    })
}

/// Re-indents the generated C by brace depth.
///
/// Codec statements nest inside each other as plain text, so their own
/// indentation carries no meaning. The generated source must stay readable
/// for the people who debug it.
fn indent(source: &str) -> String {
    let mut depth = 0usize;
    let mut indented = String::with_capacity(source.len());
    let mut previous_blank = true;
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() && previous_blank {
            continue;
        }
        previous_blank = line.is_empty();
        let (opens, closes) = braces(line);
        let leading_closes = line
            .chars()
            .take_while(|character| *character == '}')
            .count();
        let line_depth = depth.saturating_sub(leading_closes.min(closes));
        if !line.is_empty() && !line.starts_with('#') {
            (0..line_depth).for_each(|_| indented.push_str("    "));
        }
        indented.push_str(line);
        indented.push('\n');
        depth = (depth + opens).saturating_sub(closes);
    }
    indented
}

/// Counts the braces of a line of C, skipping string and character literals.
fn braces(line: &str) -> (usize, usize) {
    let mut counts = (0, 0);
    let mut quote = None;
    let mut escaped = false;
    for character in line.chars() {
        match (quote, character) {
            (Some(_), _) if escaped => escaped = false,
            (Some(_), '\\') => escaped = true,
            (Some(delimiter), _) if character == delimiter => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(character),
            (None, '{') => counts.0 += 1,
            (None, '}') => counts.1 += 1,
            (None, _) => {}
        }
    }
    counts
}
