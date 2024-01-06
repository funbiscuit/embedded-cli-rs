use darling::Result;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use crate::command::model::CommandArgs;

use super::{
    args::ArgType,
    model::{Command, CommandArg},
    TargetType,
};

pub fn derive_from_raw(target: &TargetType, commands: &[Command]) -> Result<TokenStream> {
    let ident = target.ident();

    let command_parsing = create_command_parsing(ident, commands)?;

    let named_lifetime = target.named_lifetime();

    let output = quote! {

        impl<'a> _cli::service::FromRaw<'a> for #ident #named_lifetime {
            fn parse(command: _cli::command::RawCommand<'a>) -> Result<Self, _cli::service::ParseError<'a>> {
                #command_parsing
                Ok(command)
            }
        }
    };

    Ok(output)
}

fn create_command_parsing(ident: &Ident, commands: &[Command]) -> Result<TokenStream> {
    let match_arms: Vec<_> = commands.iter().map(|c| match_arm(ident, c)).collect();

    Ok(quote! {
        let command = match command.name() {
            #(#match_arms)*
            cmd => return Err(_cli::service::ParseError::UnknownCommand),
        };
    })
}

fn match_arm(ident: &Ident, command: &Command) -> TokenStream {
    let name = command.name();
    let variant_name = command.ident();
    let variant_fqn = quote! { #ident::#variant_name };

    let rhs = match command.args() {
        CommandArgs::None => quote! { #variant_fqn, },
        CommandArgs::Named(args) => {
            let (parsing, arguments) = create_arg_parsing(args);
            quote! {
                {
                    #parsing
                    #variant_fqn { #(#arguments)* }
                }
            }
        }
    };

    quote! {  #name => #rhs }
}

fn create_arg_parsing(args: &[CommandArg]) -> (TokenStream, Vec<TokenStream>) {
    let mut variables = vec![];
    let mut arguments = vec![];
    let mut match_arms = vec![];

    for (i, arg) in args.iter().enumerate() {
        let fi = format_ident!("{}", arg.name());
        let arg_decl = quote! { #fi: };
        let ty = arg.field_type();

        let var_decl = quote! {
            let mut #fi = None;
        };

        let match_arm = quote! {
            #i => {
                let v = <#ty as _cli::arguments::FromArgument>::from_arg(arg)
                    .map_err(|_| _cli::service::ParseError::ParseArgumentError { value: arg })?;
                #fi = Some(v)
            }
        };

        //TODO: correct errors
        let var_name = match arg.ty() {
            ArgType::Option => quote! {
                #fi,
            },
            ArgType::Normal => quote! {
                #arg_decl #fi.ok_or(_cli::service::ParseError::NotEnoughArguments)?,
            },
        };
        variables.push(var_decl);
        arguments.push(var_name);
        match_arms.push(match_arm);
    }

    let parsing = quote! {
        #(#variables)*
        for (i, arg) in command.args().iter().enumerate() {
            match i {
                #(#match_arms)*
                _ => return Err(_cli::service::ParseError::TooManyArguments{
                    expected: i
                })
            }
        }
    };

    (parsing, arguments)
}
