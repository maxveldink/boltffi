use std::fmt;

use crate::{
    bridge::c,
    core::{Error, LanguageSyntax, Result, syntax::sealed},
};

/// Ruby syntax fragment family.
///
/// The Ruby target emits a C extension, so its types, expressions,
/// statements, and argument lists are the C bridge fragments. The Ruby-facing
/// names inside that C (method names, `Struct` member names, constant names)
/// and the literals in the Ruby package files use the fragments below.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Syntax;

/// A Ruby method or `Struct` member name, such as `echo_string`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Identifier(String);

/// A Ruby constant name, such as `Point`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Constant(String);

/// A Ruby constant path, such as `MyLib::Native`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ConstantPath(Vec<Constant>);

/// A double-quoted Ruby string literal.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Literal(String);

/// Methods every Ruby object answers.
///
/// A record member named `hash` would replace the hashing method.
/// The list includes public methods that Rust can spell and private initialization
/// methods and hooks that Ruby calls.
/// Ruby keywords are not in the list: `record.end` is a valid call.
const OBJECT_METHODS: &[&str] = &[
    "__id__",
    "__send__",
    "class",
    "clone",
    "define_singleton_method",
    "display",
    "dup",
    "enum_for",
    "extend",
    "freeze",
    "hash",
    "initialize",
    "initialize_clone",
    "initialize_copy",
    "initialize_dup",
    "inspect",
    "instance_eval",
    "instance_exec",
    "instance_variable_get",
    "instance_variable_set",
    "instance_variables",
    "itself",
    "method",
    "method_missing",
    "methods",
    "object_id",
    "private_methods",
    "protected_methods",
    "public_method",
    "public_methods",
    "public_send",
    "remove_instance_variable",
    "send",
    "singleton_class",
    "singleton_method",
    "singleton_method_added",
    "singleton_method_removed",
    "singleton_method_undefined",
    "singleton_methods",
    "tap",
    "then",
    "to_enum",
    "to_s",
    "yield_self",
];

/// `Struct` and `Enumerable` methods that record members must not replace.
///
/// Keep these separate from module methods.
/// A field named `size` becomes `size_`, but a module function keeps `size`.
const STRUCT_METHODS: &[&str] = &[
    "chain",
    "chunk",
    "chunk_while",
    "collect",
    "collect_concat",
    "compact",
    "count",
    "cycle",
    "deconstruct",
    "deconstruct_keys",
    "detect",
    "dig",
    "drop",
    "drop_while",
    "each",
    "each_cons",
    "each_entry",
    "each_pair",
    "each_slice",
    "each_with_index",
    "each_with_object",
    "entries",
    "filter",
    "filter_map",
    "find",
    "find_all",
    "find_index",
    "first",
    "flat_map",
    "grep",
    "grep_v",
    "group_by",
    "inject",
    "lazy",
    "length",
    "map",
    "max",
    "max_by",
    "members",
    "min",
    "min_by",
    "minmax",
    "minmax_by",
    "partition",
    "reduce",
    "reject",
    "reverse_each",
    "select",
    "size",
    "slice_after",
    "slice_before",
    "slice_when",
    "sort",
    "sort_by",
    "sum",
    "take",
    "take_while",
    "tally",
    "to_a",
    "to_h",
    "to_set",
    "uniq",
    "values",
    "values_at",
    "zip",
];

/// Methods every Ruby module answers, in addition to [`OBJECT_METHODS`].
///
/// A module function with one of these names would replace the method on the
/// generated module: a function named `name` breaks `Module#name`, and one
/// named `private` breaks `private` inside a reopened `module` body. The list
/// is `Module` public instance methods plus the private visibility methods and
/// hooks a module body calls without a receiver.
const MODULE_METHODS: &[&str] = &[
    "alias_method",
    "ancestors",
    "append_features",
    "attr",
    "attr_accessor",
    "attr_reader",
    "attr_writer",
    "autoload",
    "class_eval",
    "class_exec",
    "class_variable_get",
    "class_variable_set",
    "class_variables",
    "const_added",
    "const_get",
    "const_missing",
    "const_set",
    "const_source_location",
    "constants",
    "define_method",
    "deprecate_constant",
    "extend_object",
    "extended",
    "include",
    "included",
    "included_modules",
    "instance_method",
    "instance_methods",
    "method_added",
    "method_removed",
    "method_undefined",
    "module_eval",
    "module_exec",
    "module_function",
    "name",
    "prepend",
    "prepend_features",
    "prepended",
    "private",
    "private_class_method",
    "private_constant",
    "private_instance_methods",
    "protected",
    "protected_instance_methods",
    "public",
    "public_class_method",
    "public_constant",
    "public_instance_method",
    "public_instance_methods",
    "refine",
    "refinements",
    "remove_class_variable",
    "remove_const",
    "remove_method",
    "set_temporary_name",
    "undef_method",
    "undefined_instance_methods",
    "using",
];

impl LanguageSyntax for Syntax {
    const KEYWORDS: &'static [&'static str] = &[
        "BEGIN",
        "END",
        "__ENCODING__",
        "__FILE__",
        "__LINE__",
        "alias",
        "and",
        "begin",
        "break",
        "case",
        "class",
        "def",
        "defined?",
        "do",
        "else",
        "elsif",
        "end",
        "ensure",
        "false",
        "for",
        "if",
        "in",
        "module",
        "next",
        "nil",
        "not",
        "or",
        "redo",
        "rescue",
        "retry",
        "return",
        "self",
        "super",
        "then",
        "true",
        "undef",
        "unless",
        "until",
        "when",
        "while",
        "yield",
    ];

    type Identifier = Identifier;
    type Type = c::TypeFragment;
    type Expr = c::Expression;
    type Stmt = c::Statement;
    type Literal = Literal;
    type Arguments = c::ArgumentList;
}

impl sealed::LanguageSyntax for Syntax {}

impl Identifier {
    /// Parses a module function name.
    ///
    /// A name that every Ruby module or object already answers gets a trailing
    /// underscore, so a Rust function named `name` becomes `Demo.name_`.
    pub fn module_function(identifier: impl Into<String>) -> Result<Self> {
        Self::escape(identifier.into(), |name| {
            OBJECT_METHODS.contains(&name) || MODULE_METHODS.contains(&name)
        })
    }

    /// Parses a record member name.
    ///
    /// A name that a Ruby object or `Struct` already answers gets a trailing
    /// underscore, so a Rust field named `hash` becomes the member `hash_`.
    pub fn member(identifier: impl Into<String>) -> Result<Self> {
        Self::escape(identifier.into(), |name| {
            OBJECT_METHODS.contains(&name) || STRUCT_METHODS.contains(&name)
        })
    }

    fn escape(identifier: String, reserved: impl Fn(&str) -> bool) -> Result<Self> {
        let valid = identifier
            .chars()
            .next()
            .is_some_and(|first| first == '_' || first.is_ascii_lowercase())
            && identifier
                .chars()
                .all(|character| character == '_' || character.is_ascii_alphanumeric());
        if !valid {
            return Err(Error::InvalidRubyIdentifier { identifier });
        }
        if reserved(&identifier) {
            return Ok(Self(format!("{identifier}_")));
        }
        Ok(Self(identifier))
    }

    /// Returns the identifier text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Constant {
    /// Parses a Ruby constant name made of ASCII word characters.
    pub fn parse(constant: impl Into<String>) -> Result<Self> {
        let constant = constant.into();
        let valid = constant
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_uppercase())
            && constant
                .chars()
                .all(|character| character == '_' || character.is_ascii_alphanumeric());
        if valid {
            Ok(Self(constant))
        } else {
            Err(Error::InvalidRubyIdentifier {
                identifier: constant,
            })
        }
    }

    /// Returns the constant text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ConstantPath {
    /// Parses a `::`-separated Ruby constant path.
    pub fn parse(path: &str) -> Result<Self> {
        path.split("::")
            .map(Constant::parse)
            .collect::<Result<Vec<_>>>()
            .map(Self)
    }

    /// Creates a path with one constant.
    pub fn single(constant: Constant) -> Self {
        Self(vec![constant])
    }

    /// Returns the constants from the outermost to the innermost.
    pub fn segments(&self) -> &[Constant] {
        &self.0
    }
}

impl Literal {
    /// Creates a double-quoted Ruby string literal.
    pub fn string(value: &str) -> Self {
        let escaped = value.chars().fold(String::new(), |mut literal, character| {
            match character {
                '"' => literal.push_str("\\\""),
                '\\' => literal.push_str("\\\\"),
                '#' => literal.push_str("\\#"),
                '\n' => literal.push_str("\\n"),
                _ => literal.push(character),
            }
            literal
        });
        Self(format!("\"{escaped}\""))
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl fmt::Display for Constant {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl fmt::Display for ConstantPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = self
            .0
            .iter()
            .map(Constant::as_str)
            .collect::<Vec<_>>()
            .join("::");
        formatter.write_str(&path)
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl sealed::SyntaxFragment for Identifier {}
impl sealed::SyntaxFragment for Constant {}
impl sealed::SyntaxFragment for Literal {}

#[cfg(test)]
mod tests {
    use super::{Constant, ConstantPath, Identifier, Literal};

    #[test]
    fn names_that_shadow_core_methods_get_a_trailing_underscore() {
        assert_eq!(Identifier::member("hash").unwrap().as_str(), "hash_");
        assert_eq!(Identifier::member("name").unwrap().as_str(), "name");
        assert_eq!(
            Identifier::module_function("name").unwrap().as_str(),
            "name_"
        );
        assert_eq!(
            Identifier::module_function("hash").unwrap().as_str(),
            "hash_"
        );
        // Ruby calls this hook while `Init_` defines each module function.
        assert_eq!(
            Identifier::module_function("singleton_method_added")
                .unwrap()
                .as_str(),
            "singleton_method_added_"
        );
        assert_eq!(Identifier::module_function("end").unwrap().as_str(), "end");
        assert_eq!(
            Identifier::module_function("echo_string").unwrap().as_str(),
            "echo_string"
        );
    }

    #[test]
    fn struct_methods_are_reserved_only_for_record_members() {
        for name in ["each", "size", "values", "dig", "map", "to_h"] {
            assert_eq!(
                Identifier::member(name).unwrap().as_str(),
                format!("{name}_")
            );
            assert_eq!(Identifier::module_function(name).unwrap().as_str(), name);
        }
        assert_eq!(Identifier::member("with").unwrap().as_str(), "with");
        assert_eq!(Identifier::member("end").unwrap().as_str(), "end");
    }

    #[test]
    fn identifiers_and_constants_reject_invalid_ruby_names() {
        assert!(Identifier::member("Point").is_err());
        assert!(Identifier::module_function("has-dash").is_err());
        assert!(Constant::parse("point").is_err());
        assert!(ConstantPath::parse("MyLib::").is_err());
        assert_eq!(
            ConstantPath::parse("MyLib::Native").unwrap().to_string(),
            "MyLib::Native"
        );
    }

    #[test]
    fn string_literals_escape_interpolation() {
        assert_eq!(Literal::string("a#{b}\"c").to_string(), "\"a\\#{b}\\\"c\"");
    }
}
