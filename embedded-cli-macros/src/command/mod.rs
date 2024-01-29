use darling::{Error, FromDeriveInput, Result};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput};

use crate::{processor, utils::TargetType};

use self::model::Command;

mod args;
mod autocomplete;
#[cfg(feature = "help")]
mod doc;
mod help;
mod model;
mod parse;

#[derive(FromDeriveInput, Default)]
#[darling(default, attributes(command), forward_attrs(allow, doc, cfg))]
struct ServiceAttrs {
    help_title: Option<String>,
    skip_autocomplete: bool,
    skip_help: bool,
    skip_from_raw: bool,
}

pub fn derive_command(input: DeriveInput) -> Result<TokenStream> {
    let opts = ServiceAttrs::from_derive_input(&input)?;
    let DeriveInput {
        ident,
        data,
        generics,
        ..
    } = input;

    let data = if let Data::Enum(data) = data {
        data
    } else {
        return Err(Error::custom("Command can be derived only for an enum").with_span(&ident));
    };

    let target = TargetType::parse(ident, generics)?;

    let mut errors = Error::accumulator();
    let commands: Vec<Command> = data
        .variants
        .iter()
        .filter_map(|variant| errors.handle_in(|| Command::parse(variant)))
        .collect();
    errors.finish()?;

    let help_title = opts.help_title.unwrap_or("Commands".to_string());

    let derive_autocomplete = if opts.skip_autocomplete {
        quote! {}
    } else {
        autocomplete::derive_autocomplete(&target, &commands)?
    };
    let derive_help = if opts.skip_help {
        quote! {}
    } else {
        help::derive_help(&target, &help_title, &commands)?
    };
    let derive_from_raw = if opts.skip_from_raw {
        quote! {}
    } else {
        parse::derive_from_raw(&target, &commands)?
    };
    let impl_processor = processor::impl_processor(&target)?;

    let output = quote! {
        #derive_autocomplete

        #derive_help

        #derive_from_raw

        #impl_processor
    };

    Ok(output)
}
