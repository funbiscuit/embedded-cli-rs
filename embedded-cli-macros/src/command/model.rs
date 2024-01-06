use convert_case::{Case, Casing};
use darling::{Error, FromVariant, Result};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Field, Fields, Variant};

use super::{
    args::{ArgType, TypedArg},
    doc::Help,
};

#[derive(Debug, FromVariant, Default)]
#[darling(default, attributes(command), forward_attrs(allow, doc, cfg))]
struct CommandAttrs {
    attrs: Vec<syn::Attribute>,
    name: Option<String>,
}

pub enum CommandArgs {
    None,
    Named(Vec<CommandArg>),
}

pub struct CommandArg {
    field_name: String,
    field_type: TokenStream,
    help: Help,
    ty: ArgType,
}

impl CommandArg {
    fn parse(field: &Field) -> Result<Self> {
        let field_name = field
            .ident
            .as_ref()
            .expect("Only named fields are supported")
            .to_string();
        let aa = TypedArg::new(&field.ty);

        let ty = aa.ty();
        let field_type = aa.inner();
        let field_type = quote! { #field_type };

        let help = Help::parse(&field.attrs)?;

        Ok(Self {
            field_name,
            field_type,
            help,
            ty,
        })
    }

    pub fn help(&self) -> &Help {
        &self.help
    }

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
    help: Help,
    ident: Ident,
}

impl Command {
    pub fn parse(variant: &Variant) -> Result<Self> {
        let variant_ident = &variant.ident;
        let attrs = CommandAttrs::from_variant(variant)?;

        let help = Help::parse(&attrs.attrs)?;

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
            help,
            ident: variant_ident.clone(),
        })
    }

    pub fn args(&self) -> &CommandArgs {
        &self.args
    }

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
