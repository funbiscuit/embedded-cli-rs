use convert_case::{Case, Casing};
use darling::{Error, FromField, FromMeta, FromVariant, Result};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Field, Fields, FieldsNamed, FieldsUnnamed, Variant};

use super::args::{ArgType, TypedArg};

#[cfg(feature = "help")]
use super::doc::Help;

#[allow(dead_code)]
#[derive(Debug, FromVariant, Default)]
#[darling(default, attributes(command), forward_attrs(allow, doc, cfg))]
struct CommandAttrs {
    attrs: Vec<syn::Attribute>,
    name: Option<String>,
    subcommand: bool,
}

#[derive(Debug)]
enum LongName {
    Generated,
    Fixed(String),
}

impl FromMeta for LongName {
    fn from_string(value: &str) -> Result<Self> {
        if value.is_empty() {
            return Err(Error::custom("Name must not be empty"));
        }
        Ok(Self::Fixed(value.to_string()))
    }

    fn from_word() -> Result<Self> {
        Ok(Self::Generated)
    }
}

#[derive(Debug)]
enum ShortName {
    Generated,
    Fixed(char),
}

impl FromMeta for ShortName {
    fn from_char(value: char) -> Result<Self> {
        Ok(Self::Fixed(value))
    }

    fn from_string(value: &str) -> Result<Self> {
        let mut it = value.chars();
        let value = it
            .next()
            .ok_or(Error::custom("Short name must have single char"))?;
        if it.next().is_some() {
            return Err(Error::custom("Short name must have single char"));
        }
        Self::from_char(value)
    }

    fn from_word() -> Result<Self> {
        Ok(Self::Generated)
    }
}

#[derive(Debug, FromField, Default)]
#[darling(default, attributes(arg), forward_attrs(allow, doc, cfg))]
struct ArgAttrs {
    short: Option<ShortName>,
    long: Option<LongName>,
}

#[derive(Debug, FromField, Default)]
#[darling(default, attributes(command), forward_attrs(allow, doc, cfg))]
struct FieldCommandAttrs {
    subcommand: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub enum CommandArgType {
    /// Arg is flag and is enabled via long (--name) or short (-n) syntax.
    /// At least one of long or short is set to Some
    Flag {
        long: Option<String>,
        short: Option<char>,
    },
    /// Arg is option and is set via long (--name) or short (-n) syntax.
    /// At least one of long or short is set to Some
    Option {
        long: Option<String>,
        short: Option<char>,
    },
    Positional,
}

#[allow(unused)]
impl CommandArgType {
    pub fn is_option(&self) -> bool {
        matches!(
            self,
            CommandArgType::Flag { .. } | CommandArgType::Option { .. }
        )
    }

    pub fn is_positional(&self) -> bool {
        self == &CommandArgType::Positional
    }
}

pub struct CommandArg {
    pub arg_type: CommandArgType,
    pub field_name: String,
    pub field_type: TokenStream,
    #[cfg(feature = "help")]
    pub help: Help,
    pub ty: ArgType,
}

impl CommandArg {
    fn parse(field: &Field) -> Result<Self> {
        let arg_attrs = ArgAttrs::from_field(field)?;

        let field_name = field
            .ident
            .as_ref()
            .expect("Only named fields are supported")
            .to_string();

        let short = arg_attrs.short.map(|s| match s {
            ShortName::Generated => field_name.chars().next().unwrap(),
            ShortName::Fixed(c) => c,
        });
        if let Some(short) = short {
            if !short.is_ascii_alphabetic() {
                return Err(Error::custom("Flag char must be alphabetic ASCII"));
            }
        }

        let long = arg_attrs.long.map(|s| match s {
            LongName::Generated => field_name.from_case(Case::Snake).to_case(Case::Kebab),
            LongName::Fixed(name) => name,
        });
        if let Some(long) = &long {
            if long.chars().any(|c| !c.is_ascii_alphabetic() && c != '-') {
                return Err(Error::custom(
                    "Option name must consist of alphabetic ASCII chars",
                ));
            }
        }

        let aa = TypedArg::new(&field.ty);

        let ty = aa.ty();
        let field_type = aa.inner();
        let field_type = quote! { #field_type };
        let arg_type = if long.is_some() || short.is_some() {
            if field_type.to_string() == "bool" {
                CommandArgType::Flag { long, short }
            } else {
                CommandArgType::Option { long, short }
            }
        } else {
            CommandArgType::Positional
        };
        Ok(Self {
            arg_type,
            field_name,
            field_type,
            #[cfg(feature = "help")]
            help: Help::parse(&field.attrs)?,
            ty,
        })
    }

    pub fn full_name(&self) -> String {
        match &self.arg_type {
            CommandArgType::Flag { long, short } => long
                .as_ref()
                .map(|name| format!("--{}", name))
                .or(short.map(|n| format!("-{}", n)))
                .unwrap(),
            CommandArgType::Option { long, short } => {
                let prefix = long
                    .as_ref()
                    .map(|name| format!("--{}", name))
                    .or(short.map(|n| format!("-{}", n)))
                    .unwrap();
                if self.is_optional() {
                    format!("{} [{}]", prefix, self.field_name.to_uppercase())
                } else {
                    format!("{} <{}>", prefix, self.field_name.to_uppercase())
                }
            }
            CommandArgType::Positional => {
                if self.is_optional() {
                    format!("[{}]", self.field_name.to_uppercase())
                } else {
                    format!("<{}>", self.field_name.to_uppercase())
                }
            }
        }
    }

    pub fn is_optional(&self) -> bool {
        self.ty == ArgType::Option
    }
}

pub struct Subcommand {
    pub field_name: Option<String>,
    pub field_type: TokenStream,
    pub ty: ArgType,
}

impl Subcommand {
    fn parse_field(field: &Field) -> Result<Self> {
        let arg = TypedArg::new(&field.ty);

        let ty = arg.ty();
        let field_type = arg.inner();
        let field_type = quote! { #field_type };

        let field_name = field.ident.as_ref().map(|ident| ident.to_string());

        Ok(Self {
            field_name,
            field_type,
            ty,
        })
    }

    pub fn full_name(&self) -> String {
        if self.is_optional() {
            "[COMMAND]".to_string()
        } else {
            "<COMMAND>".to_string()
        }
    }

    pub fn is_optional(&self) -> bool {
        self.ty == ArgType::Option
    }
}

pub struct Command {
    pub name: String,
    pub args: Vec<CommandArg>,
    #[cfg(feature = "help")]
    pub help: Help,
    pub ident: Ident,
    pub named_args: bool,
    pub subcommand: Option<Subcommand>,
}

impl Command {
    pub fn parse(variant: &Variant) -> Result<Self> {
        let variant_ident = &variant.ident;
        let attrs = CommandAttrs::from_variant(variant)?;

        let (named_args, (args, subcommand)) = match &variant.fields {
            Fields::Unit => (false, (vec![], None)),
            Fields::Unnamed(fields) => (false, Self::parse_tuple_variant(&attrs, fields)?),
            Fields::Named(fields) => (true, Self::parse_struct_variant(fields)?),
        };

        let name = attrs.name.unwrap_or_else(|| {
            variant_ident
                .to_string()
                .from_case(Case::Camel)
                .to_case(Case::Kebab)
        });

        Ok(Self {
            name,
            args,
            #[cfg(feature = "help")]
            help: Help::parse(&attrs.attrs)?,
            ident: variant_ident.clone(),
            named_args,
            subcommand,
        })
    }

    fn parse_struct_variant(fields: &FieldsNamed) -> Result<(Vec<CommandArg>, Option<Subcommand>)> {
        let mut has_positional = false;
        let mut subcommand = None;

        let mut errors = Error::accumulator();
        let args = fields
            .named
            .iter()
            .filter_map(|field| {
                errors
                    .handle_in(|| {
                        let command_attrs = FieldCommandAttrs::from_field(field)?;
                        if command_attrs.subcommand {
                            if has_positional {
                                return Err(Error::custom(
                                    "Command cannot have both positional arguments and subcommand",
                                )
                                .with_span(&field.ident));
                            }
                            if subcommand.is_some() {
                                return Err(Error::custom(
                                    "Command can have only single subcommand",
                                )
                                .with_span(&field.ident));
                            }
                            subcommand = Some(Subcommand::parse_field(field)?);
                            Ok(None)
                        } else {
                            let arg = CommandArg::parse(field)?;

                            if arg.arg_type.is_positional() && subcommand.is_some() {
                                return Err(Error::custom(
                                    "Command cannot have both positional arguments and subcommand",
                                )
                                .with_span(&field.ident));
                            }
                            has_positional |= arg.arg_type.is_positional();

                            Ok(Some(arg))
                        }
                    })
                    .flatten()
            })
            .collect::<Vec<_>>();
        errors.finish()?;

        Ok((args, subcommand))
    }

    fn parse_tuple_variant(
        attrs: &CommandAttrs,
        fields: &FieldsUnnamed,
    ) -> Result<(Vec<CommandArg>, Option<Subcommand>)> {
        if fields.unnamed.len() != 1 {
            return Err(Error::custom("Tuple variant must have single argument").with_span(&fields));
        }

        if !attrs.subcommand {
            return Err(Error::custom("Tuple variant must be a subcommand").with_span(&fields));
        }

        let subcommand = Some(Subcommand::parse_field(&fields.unnamed[0])?);

        Ok((vec![], subcommand))
    }
}
