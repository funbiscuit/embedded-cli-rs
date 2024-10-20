use darling::Result;
use proc_macro2::TokenStream;
use quote::quote;

use super::{model::Command, TargetType};

#[cfg(feature = "autocomplete")]
pub fn derive_autocomplete(target: &TargetType, commands: &[Command]) -> Result<TokenStream> {
    let command_count = commands.len();
    let command_names: Vec<String> = commands.iter().map(|c| c.name.to_string()).collect();

    let ident = target.ident();
    let named_lifetime = target.named_lifetime();

    let output = quote! {
        impl #named_lifetime _cli::autocomplete::Autocomplete for #ident #named_lifetime {
            fn autocomplete(
                request: _cli::autocomplete::Request<'_>,
                autocompletion: &mut _cli::autocomplete::Autocompletion<'_>,
            ) {
                const NAMES: &[&str; #command_count] = &[#(#command_names),*];
                if let _cli::autocomplete::Request::CommandName(name) = request {
                    NAMES
                        .iter()
                        .skip_while(|n| !n.starts_with(name))
                        .take_while(|n| n.starts_with(name))
                        .for_each(|n| {
                            // SAFETY: n starts with name, so name cannot be longer
                            let autocompleted = unsafe { n.get_unchecked(name.len()..) };
                            autocompletion.merge_autocompletion(autocompleted)
                        });
                }
            }
        }
    };

    Ok(output)
}

#[allow(unused_variables)]
#[cfg(not(feature = "autocomplete"))]
pub fn derive_autocomplete(target: &TargetType, commands: &[Command]) -> Result<TokenStream> {
    let ident = target.ident();
    let named_lifetime = target.named_lifetime();

    let output = quote! {
        impl #named_lifetime _cli::autocomplete::Autocomplete for #ident #named_lifetime { }
    };

    Ok(output)
}
