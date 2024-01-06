use darling::Result;
use proc_macro2::TokenStream;
use quote::quote;

use super::{model::Command, TargetType};

pub fn derive_autocomplete(target: &TargetType, commands: &[Command]) -> Result<TokenStream> {
    let command_count = commands.len();
    let command_names: Vec<String> = commands.iter().map(|c| c.name().to_string()).collect();

    let ident = target.ident();
    let named_lifetime = target.named_lifetime();

    let output = quote! {
        impl #named_lifetime _cli::service::Autocomplete for #ident #named_lifetime {
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
                        .for_each(|n| autocompletion.merge_autocompletion(&n[name.len()..]));
                }
            }
        }
    };

    Ok(output)
}
