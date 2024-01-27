use convert_case::{Case, Casing};
use darling::{Error, FromField, FromMeta, FromVariant, Result};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Field, Fields, Variant};

use super::args::{ArgType, TypedArg};

#[cfg(feature = "help")]
use super::doc::Help;

#[allow(dead_code)]
#[derive(Debug, FromVariant, Default)]
#[darling(default, attributes(command), forward_attrs(allow, doc, cfg))]
struct CommandAttrs {
    attrs: Vec<syn::Attribute>,
    name: Option<String>,
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

pub enum CommandArgs {
    None,
    Named(Vec<CommandArg>),
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

impl CommandArgType {
    #[cfg(feature = "help")]
    pub fn is_positional(&self) -> bool {
        self == &CommandArgType::Positional
    }
}

pub struct CommandArg {
    arg_type: CommandArgType,
    field_name: String,
    field_type: TokenStream,
    #[cfg(feature = "help")]
    help: Help,
    ty: ArgType,
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

    pub fn arg_type(&self) -> &CommandArgType {
        &self.arg_type
    }

    #[cfg(feature = "help")]
    pub fn help(&self) -> &Help {
        &self.help
    }

    #[cfg(feature = "help")]
    pub fn is_optional(&self) -> bool {
        self.ty == ArgType::Option
    }

    pub fn name(&self) -> &str {
        &self.field_name
    }

    pub fn field_type(&self) -> &TokenStream {
        &self.field_type
    }

    pub fn ty(&self) -> ArgType {
        self.ty.clone()
    }
}

pub struct Command {
    name: String,
    args: CommandArgs,
    #[cfg(feature = "help")]
    help: Help,
    ident: Ident,
}

impl Command {
    pub fn parse(variant: &Variant) -> Result<Self> {
        let variant_ident = &variant.ident;
        let attrs = CommandAttrs::from_variant(variant)?;

        let name = attrs.name.unwrap_or_else(|| {
            variant_ident
                .to_string()
                .from_case(Case::Camel)
                .to_case(Case::Kebab)
        });

        let args = match &variant.fields {
            Fields::Unit => CommandArgs::None,
            Fields::Unnamed(fields) => {
                return Err(Error::custom(
                    "Unnamed/tuple fields are not supported. Use named fields",
                )
                .with_span(fields));
            }
            Fields::Named(fields) => {
                let mut errors = Error::accumulator();
                let args = fields
                    .named
                    .iter()
                    .filter_map(|field| errors.handle_in(|| CommandArg::parse(field)))
                    .collect::<Vec<_>>();
                errors.finish()?;

                CommandArgs::Named(args)
            }
        };

        Ok(Self {
            name,
            args,
            #[cfg(feature = "help")]
            help: Help::parse(&attrs.attrs)?,
            ident: variant_ident.clone(),
        })
    }

    pub fn args(&self) -> &CommandArgs {
        &self.args
    }

    #[cfg(feature = "help")]
    pub fn help(&self) -> &Help {
        &self.help
    }

    pub fn ident(&self) -> &Ident {
        &self.ident
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
