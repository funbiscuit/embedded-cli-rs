use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

mod command;
mod group;
mod utils;

#[proc_macro_derive(Command, attributes(command, arg))]
pub fn derive_command(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);

    let output = match command::derive_command(input) {
        Ok(output) => output,
        Err(e) => return e.write_errors().into(),
    };

    // wrap with anonymous scope
    quote! {
        const _: () = {
            extern crate embedded_cli as _cli;
            use _cli::__private::io as _io;

            #output
        };
    }
    .into()
}

#[proc_macro_derive(CommandGroup, attributes(group))]
pub fn derive_command_group(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);

    let output = match group::derive_command_group(input) {
        Ok(output) => output,
        Err(e) => return e.write_errors().into(),
    };

    // wrap with anonymous scope
    quote! {
        const _: () = {
            extern crate embedded_cli as _cli;
            use _cli::__private::io as _io;

            #output
        };
    }
    .into()
}
