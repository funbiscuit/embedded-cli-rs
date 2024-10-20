use convert_case::{Case, Casing};
use darling::Result;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use super::{
    args::ArgType,
    model::{Command, CommandArgType},
    TargetType,
};

pub fn derive_from_command(target: &TargetType, commands: &[Command]) -> Result<TokenStream> {
    let ident = target.ident();

    let parsing = create_parsing(ident, commands)?;

    let named_lifetime = target.named_lifetime();

    let output = quote! {

        impl<'a> _cli::command::FromCommand<'a> for #ident #named_lifetime {
            fn parse(name: &'a str, args: _cli::arguments::Args<'a>) -> Result<Self, _cli::command::ParseError<'a>> {
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
        let command = match name {
            #(#match_arms)*
            _ => return Err(_cli::command::ParseError::UnknownCommand),
        };
    })
}

fn command_parsing(ident: &Ident, command: &Command) -> TokenStream {
    let name = &command.name;
    let variant_name = &command.ident;
    let variant_fqn = quote! { #ident::#variant_name };

    let rhs = if command.args.is_empty() && command.subcommand.is_none() {
        quote! { #variant_fqn, }
    } else {
        let (parsing, arguments) = create_arg_parsing(command);
        if command.named_args {
            quote! {
                {
                    #parsing
                    #variant_fqn { #(#arguments)* }
                }
            }
        } else {
            quote! {
                {
                    #parsing
                    #variant_fqn ( #(#arguments)* )
                }
            }
        }
    };

    quote! {  #name => #rhs }
}

fn create_arg_parsing(command: &Command) -> (TokenStream, Vec<TokenStream>) {
    let mut variables = vec![];
    let mut arguments = vec![];
    let mut positional_value_arms = vec![];
    let mut extra_states = vec![];
    let mut option_name_arms = vec![];
    let mut option_value_arms = vec![];

    let mut positional = 0usize;
    for arg in &command.args {
        let fi_raw = format_ident!("{}", arg.field_name);
        let fi = format_ident!("arg_{}", arg.field_name);
        let ty = &arg.field_type;

        let arg_default;

        match &arg.arg_type {
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
                arg_default = arg.default_value.clone();
                let state = format_ident!(
                    "Expect{}",
                    arg.field_name.from_case(Case::Snake).to_case(Case::Pascal)
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
                arg_default = arg.default_value.clone();
                let parse_value = create_parse_arg_value(ty);

                positional_value_arms.push(quote! {
                    #positional => {
                        #fi = Some(#parse_value);
                    },
                });
                positional += 1;
            }
        }

        let constructor_arg = match arg.ty {
            ArgType::Option => quote! { #fi_raw: #fi },
            ArgType::Normal => {
                if let Some(default) = arg_default {
                    quote! {
                        #fi_raw: #fi.unwrap_or(#default)
                    }
                } else {
                    let name = arg.full_name();
                    quote! {
                        #fi_raw: #fi.ok_or(_cli::command::ParseError::MissingRequiredArgument {
                            name: #name,
                        })?
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

    let subcommand_value_arm;
    if let Some(subcommand) = &command.subcommand {
        let fi_raw;
        let fi;
        if let Some(field_name) = &subcommand.field_name {
            let ident_raw = format_ident!("{}", field_name);
            fi_raw = quote! { #ident_raw: };
            fi = format_ident!("sub_{}", field_name);
        } else {
            fi_raw = quote! {};
            fi = format_ident!("sub_command");
        }
        let ty = &subcommand.field_type;

        subcommand_value_arm = Some(quote! {
            let args = args.into_args();
            #fi = Some(<#ty as _cli::command::FromCommand>::parse(name, args)?);
            break;
        });

        let constructor_arg = match subcommand.ty {
            ArgType::Option => quote! { #fi_raw #fi },
            ArgType::Normal => {
                let name = subcommand.full_name();
                quote! {
                    #fi_raw #fi.ok_or(_cli::command::ParseError::MissingRequiredArgument {
                        name: #name,
                    })?
                }
            }
        };

        variables.push(quote! {
            let mut #fi = None;
        });
        arguments.push(quote! {
            #constructor_arg,
        });
    } else {
        subcommand_value_arm = None;
    }

    let value_arm = if let Some(subcommand_arm) = subcommand_value_arm {
        quote! {
            _cli::arguments::Arg::Value(name) if state == States::Normal => {
                #subcommand_arm
            }
        }
    } else if positional_value_arms.is_empty() {
        quote! {
            _cli::arguments::Arg::Value(value) if state == States::Normal =>
            return Err(_cli::command::ParseError::UnexpectedArgument {
                value,
            })
        }
    } else {
        quote! {
            _cli::arguments::Arg::Value(val) if state == States::Normal => {
                match positional {
                    #(#positional_value_arms)*
                    _ => return Err(_cli::command::ParseError::UnexpectedArgument{
                        value: val
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

        let mut args = args.iter();
        while let Some(arg) = args.next() {
            match arg {
                #(#option_name_arms)*
                #(#option_value_arms)*
                #value_arm,
                _cli::arguments::Arg::Value(_) => unreachable!(),
                _cli::arguments::Arg::LongOption(option) => {
                    return Err(_cli::command::ParseError::UnexpectedLongOption { name: option })
                }
                _cli::arguments::Arg::ShortOption(option) => {
                    return Err(_cli::command::ParseError::UnexpectedShortOption { name: option })
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
        <#ty as _cli::arguments::FromArg>::from_arg(val)?,
    }
}
