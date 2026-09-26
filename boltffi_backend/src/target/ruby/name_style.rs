use boltffi_binding::{CanonicalName, FieldKey, NamePart};

use crate::core::{Error, Result, name_case};

use super::syntax::{Constant, ConstantPath, Identifier};

/// Ruby spelling of one canonical binding name.
pub struct Name<'name> {
    source: &'name CanonicalName,
}

impl<'name> Name<'name> {
    pub fn new(source: &'name CanonicalName) -> Self {
        Self { source }
    }

    /// The `snake_case` module function name, such as `echo_string`.
    pub fn module_function(&self) -> Result<Identifier> {
        Identifier::module_function(self.snake())
    }

    /// The `UpperCamelCase` constant name, such as `Point`.
    pub fn constant(&self) -> Result<Constant> {
        Constant::parse(name_case::upper_camel(self.source))
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

/// The default Ruby module for a Cargo package, such as `CheckoutEngine` for
/// `checkout-engine`.
pub fn default_module(package: &CanonicalName) -> Result<ConstantPath> {
    Constant::parse(name_case::upper_camel_from_snake(&package_snake(package)))
        .map(ConstantPath::single)
}

/// The `snake_case` spelling of a Cargo package, such as `checkout_engine`.
///
/// The binding contract keeps the Cargo package name as one name part, dashes
/// included, so the dashes become underscores here.
pub fn package_snake(package: &CanonicalName) -> String {
    extension_stem(&Name::new(package).snake())
}

/// The extension file stem for a gem, such as `checkout_engine` for
/// `checkout-engine`.
///
/// The stem names the compiled library and its `Init_<stem>` entry point, so
/// it keeps only ASCII lowercase letters, digits, and underscores.
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

    use super::{default_module, extension_stem, package_snake};

    #[test]
    fn package_names_map_to_ruby_modules_and_extension_stems() {
        for package in [
            CanonicalName::single("checkout-engine"),
            CanonicalName::single("checkout_engine"),
            CanonicalName::new(vec![NamePart::new("checkout"), NamePart::new("engine")]),
        ] {
            assert_eq!(package_snake(&package), "checkout_engine");
            assert_eq!(
                default_module(&package).unwrap().to_string(),
                "CheckoutEngine"
            );
        }
        assert_eq!(extension_stem("checkout-engine"), "checkout_engine");
    }
}
