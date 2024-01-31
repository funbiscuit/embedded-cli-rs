use convert_case::{Case, Casing};
use darling::Result;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use crate::command::model::CommandArgs;

use super::{
    args::ArgType,
    model::{Command, CommandArg, CommandArgType},
    TargetType,
};

pub fn derive_from_raw(target: &TargetType, commands: &[Command]) -> Result<TokenStream> {
    let ident = target.ident();

    let parsing = create_parsing(ident, commands)?;

    let named_lifetime = target.named_lifetime();

    let output = quote! {

        impl<'a> _cli::service::FromRaw<'a> for #ident #named_lifetime {
            fn parse(command: _cli::command::RawCommand<'a>) -> Result<Self, _cli::service::ParseError<'a>> {
                #parsing
                Ok(command)
            }
        }
    };

    Ok(output)
}

fn create_parsing(ident: &Ident, commands: &[Command]) -> Result<TokenStream> {
    let match_arms: Vec<_> = commands.iter().map(|c| command_parsing(ident, c)).collect();

    Ok(quote! {
        let command = match command.name() {
            #(#match_arms)*
            cmd => return Err(_cli::service::ParseError::UnknownCommand),
        };
    })
}

fn command_parsing(ident: &Ident, command: &Command) -> TokenStream {
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
    let mut positional_value_arms = vec![];
    let mut extra_states = vec![];
    let mut option_name_arms = vec![];
    let mut option_value_arms = vec![];
    let mut subcommand_value_arm = None;

    let mut positional = 0usize;
    for arg in args.iter() {
        let fi_raw = format_ident!("{}", arg.name());
        let fi = format_ident!("arg_{}", arg.name());
        let ty = arg.field_type();

        let arg_default;

        match arg.arg_type() {
            CommandArgType::Flag { long, short } => {
                arg_default = Some(quote! { false });

                option_name_arms.push(create_option_name_arm(
                    short,
                    long,
                    quote! {
                        {
                            #fi = Some(true);
                            state = States::Normal;
                        }
                    },
                ));
            }
            CommandArgType::Option { long, short } => {
                arg_default = None;
                let state = format_ident!(
                    "Expect{}",
                    arg.name().from_case(Case::Snake).to_case(Case::Pascal)
                );
                extra_states.push(quote! { #state, });

                let parse_value = create_parse_arg_value(ty);
                option_value_arms.push(quote! {
                    _cli::arguments::Arg::Value(val) if state == States::#state => {
                        #fi = Some(#parse_value);
                        state = States::Normal;
                    }
                });

                option_name_arms.push(create_option_name_arm(
                    short,
                    long,
                    quote! { state = States::#state },
                ));
            }
            CommandArgType::Positional => {
                arg_default = None;
                let parse_value = create_parse_arg_value(ty);

                positional_value_arms.push(quote! {
                    #positional => {
                        #fi = Some(#parse_value);
                    },
                });
                positional += 1;
            }
            CommandArgType::SubCommand => {
                // in model we checked that subcommand is only one and there is no positional args
                arg_default = None;

                subcommand_value_arm = Some(quote! {
                    let args = args.into_args();
                    let raw = _cli::command::RawCommand::new(name, args);

                    #fi = Some(<#ty as _cli::service::FromRaw>::parse(raw)?);

                    break;
                });
            }
        }

        //TODO: correct errors
        let constructor_arg = match arg.ty() {
            ArgType::Option => quote! { #fi_raw: #fi },
            ArgType::Normal => {
                if let Some(default) = arg_default {
                    quote! {
                        #fi_raw: #fi.unwrap_or(#default)
                    }
                } else {
                    quote! {
                        #fi_raw: #fi.ok_or(_cli::service::ParseError::NotEnoughArguments)?
                    }
                }
            }
        };

        variables.push(quote! {
            let mut #fi = None;
        });
        arguments.push(quote! {
            #constructor_arg,
        });
    }

    let value_arm = if let Some(subcommand_arm) = subcommand_value_arm {
        quote! {
            _cli::arguments::Arg::Value(name) if state == States::Normal => {
                #subcommand_arm
            }
        }
    } else if positional_value_arms.is_empty() {
        quote! {
            _cli::arguments::Arg::Value(_) if state == States::Normal =>
            return Err(_cli::service::ParseError::TooManyArguments{
                expected: positional
            })
        }
    } else {
        quote! {
            _cli::arguments::Arg::Value(val) if state == States::Normal => {
                match positional {
                    #(#positional_value_arms)*
                    _ => return Err(_cli::service::ParseError::TooManyArguments{
                        expected: positional
                    })
                }
                positional += 1;
            }
        }
    };

    let parsing = quote! {
        #(#variables)*

        #[derive(Eq, PartialEq)]
        enum States {
            Normal,
            #(#extra_states)*
        }
        let mut state = States::Normal;
        let mut positional = 0;

        let mut args = command.args().args();
        while let Some(arg) = args.next() {
            let arg = arg.map_err(|_| _cli::service::ParseError::Other(""))?;
            match arg {
                #(#option_name_arms)*
                #(#option_value_arms)*
                #value_arm,
                _cli::arguments::Arg::Value(_) => unreachable!(),
                _cli::arguments::Arg::LongOption(option) => {
                    return Err(_cli::service::ParseError::UnknownOption { name: option })
                }
                _cli::arguments::Arg::ShortOption(option) => {
                    return Err(_cli::service::ParseError::UnknownFlag { flag: option })
                }
                _cli::arguments::Arg::DoubleDash => {}
            }
        }
    };

    (parsing, arguments)
}

pub fn create_option_name_arm(
    short: &Option<char>,
    long: &Option<String>,
    action: TokenStream,
) -> TokenStream {
    match (short, long) {
        (Some(short), Some(long)) => {
            quote! {
                _cli::arguments::Arg::LongOption(#long)
                | _cli::arguments::Arg::ShortOption(#short) => #action,
            }
        }
        (Some(short), None) => {
            quote! {
                _cli::arguments::Arg::ShortOption(#short) => #action,
            }
        }
        (None, Some(long)) => {
            quote! {
                _cli::arguments::Arg::LongOption(#long) => #action,
            }
        }
        (None, None) => unreachable!(),
    }
}

fn create_parse_arg_value(ty: &TokenStream) -> TokenStream {
    quote! {
        <#ty as _cli::arguments::FromArgument>::from_arg(val).map_err(|_|
            _cli::service::ParseError::ParseArgumentError { value: val }
        )?,
    }
}
