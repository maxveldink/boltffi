use std::collections::{HashMap, hash_map::Entry};

use boltffi_binding::{CanonicalName, FieldKey, NamePart};

use crate::core::{Error, Result, name_case};

use super::syntax::{Constant, ConstantPath, Identifier};

/// Ruby spelling of one canonical binding name.
pub struct Name<'name> {
    source: &'name CanonicalName,
}

impl<'name> Name<'name> {
    /// Spells `source` in Ruby.
    pub fn new(source: &'name CanonicalName) -> Self {
        Self { source }
    }

    /// The `snake_case` module function name, such as `echo_string`.
    pub fn module_function(&self) -> Result<Identifier> {
        Identifier::module_function(self.snake())
    }

    /// The `UpperCamelCase` constant name, such as `Point`.
    ///
    /// A keyword spelling such as `END` becomes `END_`.
    pub fn constant(&self) -> Result<Constant> {
        Constant::declaration(name_case::upper_camel(self.source))
    }

    fn snake(&self) -> String {
        self.source
            .parts()
            .iter()
            .map(NamePart::as_str)
            .collect::<Vec<_>>()
            .join("_")
    }
}

/// The Rust spelling of a declaration name, such as `hash_`, for diagnostics.
pub fn spelling(name: &CanonicalName) -> String {
    name.source_spelling()
        .map(str::to_owned)
        .unwrap_or_else(|| Name::new(name).snake())
}

/// Generated Ruby names in one scope, with the declaration that took each.
///
/// Escaping can map two Rust names to one Ruby name: a field `hash` becomes
/// the member `hash_`, which a field `hash_` also wants.
pub struct NameScope {
    scope: String,
    names: HashMap<String, String>,
}

impl NameScope {
    /// Creates an empty scope named `scope` for collision errors.
    pub fn new(scope: impl Into<String>) -> Self {
        Self {
            scope: scope.into(),
            names: HashMap::new(),
        }
    }

    /// Claims `name` for `subject`, such as ``field `hash_` ``.
    pub fn claim(&mut self, name: &str, subject: String) -> Result<()> {
        match self.names.entry(name.to_owned()) {
            Entry::Vacant(entry) => {
                entry.insert(subject);
                Ok(())
            }
            Entry::Occupied(entry) => Err(Error::RubyNameCollision {
                scope: self.scope.clone(),
                name: name.to_owned(),
                existing: entry.get().clone(),
                colliding: subject,
            }),
        }
    }
}

/// The `Data` member name for one record field.
///
/// Tuple fields have no name in Rust, so field `0` becomes `field_0`.
pub fn member(key: &FieldKey) -> Result<Identifier> {
    match key {
        FieldKey::Named(name) => Identifier::member(Name::new(name).snake()),
        FieldKey::Position(position) => Identifier::member(format!("field_{position}")),
        _ => Err(Error::UnsupportedTarget {
            target: "ruby",
            shape: "unknown record field key",
        }),
    }
}

/// The default Ruby module for a Cargo package, such as `MyLib` for
/// `my-lib`.
pub fn default_module(package: &CanonicalName) -> Result<ConstantPath> {
    Constant::parse(name_case::upper_camel_from_snake(&package_snake(package)))
        .map(ConstantPath::single)
}

/// The `snake_case` spelling of a Cargo package, such as `my_lib`.
///
/// The binding contract keeps the Cargo package name as one name part, dashes
/// included, so the dashes become underscores here.
pub fn package_snake(package: &CanonicalName) -> String {
    extension_stem(&Name::new(package).snake())
}

/// The extension file stem for a gem, such as `my_lib` for `my-lib`.
///
/// The stem names the extension directory, the compiled library, and its
/// `Init_<stem>` entry point, so it keeps only ASCII lowercase letters,
/// digits, and underscores.
pub fn extension_stem(gem: &str) -> String {
    gem.chars()
        .map(|character| match character {
            'a'..='z' | '0'..='9' | '_' => character,
            'A'..='Z' => character.to_ascii_lowercase(),
            _ => '_',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use boltffi_binding::{CanonicalName, NamePart};

    use super::{Name, default_module, extension_stem, package_snake};

    #[test]
    fn package_names_map_to_ruby_modules_and_extension_stems() {
        for package in [
            CanonicalName::single("my-lib"),
            CanonicalName::single("my_lib"),
            CanonicalName::new(vec![NamePart::new("my"), NamePart::new("lib")]),
        ] {
            assert_eq!(package_snake(&package), "my_lib");
            assert_eq!(default_module(&package).unwrap().to_string(), "MyLib");
        }
        assert_eq!(extension_stem("my-lib"), "my_lib");
    }

    #[test]
    fn declaration_constants_escape_keyword_spellings() {
        for (spelling, expected) in [("END", "END_"), ("BEGIN", "BEGIN_"), ("HTTP", "HTTP")] {
            let name =
                CanonicalName::from_source(spelling, vec![NamePart::new(spelling.to_lowercase())]);
            assert_eq!(Name::new(&name).constant().unwrap().as_str(), expected);
        }
    }
}
