use darling::{Error, Result};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput};

use crate::utils::TargetType;

use self::command_group::CommandGroup;

mod command_group;

pub fn derive_command_group(input: DeriveInput) -> Result<TokenStream> {
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
    let groups: Vec<CommandGroup> = data
        .variants
        .iter()
        .filter_map(|variant| errors.handle_in(|| CommandGroup::parse(variant)))
        .collect();
    errors.finish()?;

    let derive_autocomplete = derive_autocomplete(&target, &groups);
    let derive_help = derive_help(&target, &groups);
    let derive_from_raw = derive_from_raw(&target, &groups);

    let output = quote! {
        #derive_autocomplete
        #derive_help
        #derive_from_raw
    };

    Ok(output)
}

#[cfg(feature = "autocomplete")]
fn derive_autocomplete(target: &TargetType, groups: &[CommandGroup]) -> TokenStream {
    let ident = target.ident();
    let named_lifetime = target.named_lifetime();

    let groups = groups
        .iter()
        .filter(|group| !group.hidden)
        .map(|group| {
            let ty = &group.field_type;
            quote! {
                <#ty as _cli::service::Autocomplete>::autocomplete(request.clone(), autocompletion);
            }
        })
        .collect::<Vec<_>>();

    quote! {
        impl #named_lifetime _cli::service::Autocomplete for #ident #named_lifetime {
            fn autocomplete(
                request: _cli::autocomplete::Request<'_>,
                autocompletion: &mut _cli::autocomplete::Autocompletion<'_>,
            ) {
                #(#groups)*
            }
        }
    }
}

#[allow(unused_variables)]
#[cfg(not(feature = "autocomplete"))]
fn derive_autocomplete(target: &TargetType, groups: &[CommandGroup]) -> TokenStream {
    let ident = target.ident();
    let named_lifetime = target.named_lifetime();

    quote! {
        impl #named_lifetime _cli::service::Autocomplete for #ident #named_lifetime { }
    }
}

#[cfg(feature = "help")]
fn derive_help(target: &TargetType, groups: &[CommandGroup]) -> TokenStream {
    let ident = target.ident();
    let named_lifetime = target.named_lifetime();

    let command_counts = groups
        .iter()
        .filter(|group| !group.hidden)
        .enumerate()
        .map(|(i, group)| {
            let ty = &group.field_type;
            if i > 0 {
                quote! {
                    + <#ty as _cli::service::Help>::command_count()
                }
            } else {
                quote! {
                    <#ty as _cli::service::Help>::command_count()
                }
            }
        })
        .collect::<Vec<_>>();

    let command_help = groups
        .iter()
        .filter(|group| !group.hidden)
        .enumerate()
        .map(|(i, group)| {
            let ty = &group.field_type;
            if i > 0 {
                quote! {
                    .or_else(|_| <#ty as _cli::service::Help>::command_help(parent, command.clone(), writer))
                }
            } else {
                quote! {
                    <#ty as _cli::service::Help>::command_help(parent, command.clone(), writer)
                }
            }
        })
        .collect::<Vec<_>>();

    let list_commands = groups
        .iter()
        .filter(|group| !group.hidden)
        .map(|group| {
            let ty = &group.field_type;
            quote! {
                if <#ty as _cli::service::Help>::command_count() > 0 {
                    if has_output {
                        writer.writeln_str("")?;
                    }
                    <#ty as _cli::service::Help>::list_commands(writer)?;
                    has_output = true;
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        impl #named_lifetime _cli::service::Help for #ident #named_lifetime {

            fn command_count() -> usize {
                #(#command_counts)*
            }

            fn list_commands<W: _io::Write<Error = E>, E: _io::Error>(
                writer: &mut _cli::writer::Writer<'_, W, E>,
            ) -> Result<(), E> {
                let mut has_output = false;
                #(#list_commands)*
                Ok(())
            }

            fn command_help<
                W: _io::Write<Error = E>,
                E: _io::Error,
                F: FnMut(&mut _cli::writer::Writer<'_, W, E>) -> Result<(), E>,
            >(
                parent: &mut F,
                command: _cli::command::RawCommand<'_>,
                writer: &mut _cli::writer::Writer<'_, W, E>,
            ) -> Result<(), _cli::service::HelpError<E>> {
                #(#command_help)*?;
                Ok(())
            }
        }
    }
}

#[allow(unused_variables)]
#[cfg(not(feature = "help"))]
fn derive_help(target: &TargetType, groups: &[CommandGroup]) -> TokenStream {
    let ident = target.ident();
    let named_lifetime = target.named_lifetime();

    quote! {
        impl #named_lifetime _cli::service::Help for #ident #named_lifetime { }
    }
}

fn derive_from_raw(target: &TargetType, groups: &[CommandGroup]) -> TokenStream {
    let ident = target.ident();
    let named_lifetime = target.named_lifetime();

    let groups = groups
        .iter()
        .map(|group| {
            let ident = &group.ident;
            let ty = &group.field_type;
            quote! {
                match <#ty as _cli::service::FromRaw>::parse(raw.clone()) {
                    Ok(cmd) => {
                        return Ok(Self:: #ident (cmd));
                    }
                    Err(_cli::service::ParseError::UnknownCommand) => {}
                    Err(err) => return Err(err),
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        impl<'a> _cli::service::FromRaw<'a> for #ident #named_lifetime {
            fn parse(raw: _cli::command::RawCommand<'a>) -> Result<Self, _cli::service::ParseError<'a>> {
                #(#groups)*

                Err(_cli::service::ParseError::UnknownCommand)
            }
        }
    }
}
