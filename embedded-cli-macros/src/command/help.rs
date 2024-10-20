use darling::Result;
use proc_macro2::TokenStream;
use quote::quote;

use super::{model::Command, TargetType};

#[cfg(feature = "help")]
use super::model::{CommandArg, CommandArgType};

#[cfg(feature = "help")]
pub fn derive_help(
    target: &TargetType,
    help_title: &str,
    commands: &[Command],
) -> Result<TokenStream> {
    let list_commands = create_help_all(commands, help_title)?;
    let commands_help = commands.iter().map(create_command_help).collect::<Vec<_>>();

    let ident = target.ident();
    let named_lifetime = target.named_lifetime();
    let command_count = commands.len();

    let output = quote! {
        impl #named_lifetime _cli::help::Help for #ident #named_lifetime {
            fn command_count() -> usize { #command_count }

            fn list_commands<W: _io::Write<Error = E>, E: _io::Error>(
                writer: &mut _cli::writer::Writer<'_, W, E>,
            ) -> Result<(), E> {
                #list_commands
                Ok(())
            }

            fn command_help<
                W: _io::Write<Error = E>,
                E: _io::Error,
                F: FnMut(&mut _cli::writer::Writer<'_, W, E>) -> Result<(), E>,
            >(
                parent: &mut F,
                name: &str,
                args: _cli::arguments::Args<'_>,
                writer: &mut _cli::writer::Writer<'_, W, E>,
            ) -> Result<(), _cli::help::HelpError<E>> {
                match name {
                    #(#commands_help)*
                    _ => return Err(_cli::help::HelpError::UnknownCommand),
                }

                Ok(())
            }
        }
    };

    Ok(output)
}

#[allow(unused_variables)]
#[cfg(not(feature = "help"))]
pub fn derive_help(
    target: &TargetType,
    help_title: &str,
    commands: &[Command],
) -> Result<TokenStream> {
    let ident = target.ident();
    let named_lifetime = target.named_lifetime();

    let output = quote! {
        impl #named_lifetime _cli::help::Help for #ident #named_lifetime { }
    };

    Ok(output)
}

#[cfg(feature = "help")]
fn create_help_all(commands: &[Command], title: &str) -> Result<TokenStream> {
    let max_len = commands.iter().map(|c| c.name.len()).max().unwrap_or(0);
    let elements: Vec<_> = commands
        .iter()
        .map(|c| {
            let name = &c.name;
            let help = c.help.short().unwrap_or("");
            quote! {
                writer.write_list_element(#name, #help, #max_len)?;
            }
        })
        .collect();

    let title = format!("{}:", title);
    Ok(quote! {
        writer.write_title(#title)?;
        writer.writeln_str("")?;
        #(#elements)*
    })
}

#[cfg(feature = "help")]
fn create_command_help(command: &Command) -> TokenStream {
    use convert_case::{Case, Casing};
    use quote::format_ident;

    use crate::command::parse;

    let name = &command.name;

    let help = command.help.long().map(|help| {
        quote! { writer.writeln_str(#help)?; }
    });

    let usage = create_usage(name, command);
    let args_help = create_args_help(&command.args);
    let options_help = create_options_help(&command.args);
    let commands_help = create_commands_help(command);

    let blocks = help
        .into_iter()
        .chain(Some(usage))
        .chain(args_help)
        .chain(Some(options_help))
        .chain(commands_help)
        .reduce(|acc, elem| {
            quote! {
                #acc
                writer.writeln_str("")?;
                #elem
            }
        })
        .unwrap();

    if let Some(subcommand) = &command.subcommand {
        let mut extra_states = vec![];
        let mut option_name_arms = vec![];
        let mut option_value_arms = vec![];

        for arg in &command.args {
            match &arg.arg_type {
                CommandArgType::Flag { long, short } => {
                    option_name_arms.push(parse::create_option_name_arm(
                        short,
                        long,
                        quote! {
                            {
                                state = States::Normal;
                            }
                        },
                    ));
                }
                CommandArgType::Option { long, short } => {
                    let state = format_ident!(
                        "Expect{}",
                        arg.field_name.from_case(Case::Snake).to_case(Case::Pascal)
                    );
                    extra_states.push(quote! { #state, });

                    option_value_arms.push(quote! {
                        _cli::arguments::Arg::Value(val) if state == States::#state => {
                            state = States::Normal;
                        }
                    });

                    option_name_arms.push(parse::create_option_name_arm(
                        short,
                        long,
                        quote! { state = States::#state },
                    ));
                }
                CommandArgType::Positional => {
                    unreachable!("command with subcommand doesn't have positional args")
                }
            }
        }

        let subcommand_ty = &subcommand.field_type;
        let subcommand_value_arm = quote! {
            let args = args_iter.into_args();

            let mut parent = |writer: &mut _cli::writer::Writer<'_, W, E>| {
                parent(writer)?;
                writer.write_str(#name)?;
                writer.write_str(" ")?;
                Ok(())
            };

            return <#subcommand_ty as _cli::help::Help>::command_help(&mut parent, name, args, writer);
        };

        let value_arm = quote! {
            _cli::arguments::Arg::Value(name) if state == States::Normal => {
                #subcommand_value_arm
            }
        };

        quote! {
            #name => {
                #[derive(Eq, PartialEq)]
                enum States {
                    Normal,
                    #(#extra_states)*
                }
                let mut state = States::Normal;

                let mut args_iter = args.iter();
                while let Some(arg) = args_iter.next() {
                    match arg {
                        #(#option_name_arms)*
                        #(#option_value_arms)*
                        #value_arm,
                        _cli::arguments::Arg::Value(_) => unreachable!(),
                        _cli::arguments::Arg::LongOption(_) | _cli::arguments::Arg::ShortOption(_) => break,
                        _cli::arguments::Arg::DoubleDash => {}
                    }
                }

                #blocks
            },
        }
    } else {
        quote! {
            #name => {
                #blocks
            },
        }
    }
}

#[cfg(feature = "help")]
fn create_args_help(args: &[CommandArg]) -> Option<TokenStream> {
    let longest_arg = args
        .iter()
        .filter(|a| a.arg_type.is_positional())
        .map(|a| a.full_name().len())
        .max()
        .unwrap_or(0);

    let help_lines = args
        .iter()
        .filter_map(|arg| match &arg.arg_type {
            CommandArgType::Positional => {
                let name = arg.full_name();

                let arg_help = arg.help.short().unwrap_or("");

                Some(quote! {
                    writer.write_list_element(#name, #arg_help, #longest_arg)?;
                })
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    if help_lines.is_empty() {
        None
    } else {
        Some(quote! {
           writer.write_title("Arguments:\n")?;
           #(#help_lines)*
        })
    }
}

#[cfg(feature = "help")]
fn create_commands_help(command: &Command) -> Option<TokenStream> {
    command.subcommand.as_ref().map(|subcommand| {
        let ty = &subcommand.field_type;
        quote! {
            <#ty as _cli::help::Help>::list_commands(writer)?;
        }
    })
}

#[cfg(feature = "help")]
fn create_options_help(args: &[CommandArg]) -> TokenStream {
    struct OptionHelp {
        name: String,
        help: String,
    }

    let mut help_lines = args
        .iter()
        .filter_map(|arg| match &arg.arg_type {
            CommandArgType::Flag { long, short } => {
                let name = short
                    .map(|name| format!("-{}", name))
                    .into_iter()
                    .chain(long.iter().map(|name| format!("--{}", name)))
                    .collect::<Vec<_>>()
                    .join(", ");

                let help = arg.help.short().unwrap_or("").to_string();

                Some(OptionHelp { name, help })
            }
            CommandArgType::Option { long, short } => {
                let name = short
                    .map(|name| format!("-{}", name))
                    .into_iter()
                    .chain(long.iter().map(|name| format!("--{}", name)))
                    .collect::<Vec<_>>()
                    .join(", ");

                let value = if arg.is_optional() {
                    format!("[{}]", arg.value_name)
                } else {
                    format!("<{}>", arg.value_name)
                };

                let name = format!("{} {}", name, value);

                let help = arg.help.short().unwrap_or("").to_string();

                Some(OptionHelp { name, help })
            }
            CommandArgType::Positional => None,
        })
        .collect::<Vec<_>>();

    help_lines.push(OptionHelp {
        name: "-h, --help".to_string(),
        help: "Print help".to_string(),
    });
    let longest_name = help_lines.iter().map(|a| a.name.len()).max().unwrap();

    let help_lines = help_lines
        .into_iter()
        .map(|help| {
            let name = help.name;
            let help = help.help;
            quote! {
                writer.write_list_element(#name, #help, #longest_name)?;
            }
        })
        .collect::<Vec<_>>();

    quote! {
        writer.write_title("Options:")?;
        writer.writeln_str("")?;
        #(#help_lines)*
    }
}

#[cfg(feature = "help")]
fn create_usage(name: &str, command: &Command) -> TokenStream {
    let args = &command.args;
    let has_options = args.iter().any(|arg| arg.arg_type.is_option());

    let usage_args;
    if let Some(subcommand) = &command.subcommand {
        if subcommand.is_optional() {
            usage_args = vec![quote! {
                writer.write_str(" [COMMAND]")?;
            }]
        } else {
            usage_args = vec![quote! {
                writer.write_str(" <COMMAND>")?;
            }]
        }
    } else {
        usage_args = args
            .iter()
            .filter_map(|arg| match &arg.arg_type {
                CommandArgType::Positional => Some(arg.full_name()),
                _ => None,
            })
            .map(|line| {
                quote! {
                    writer.write_str(" ")?;
                    writer.write_str(#line)?;
                }
            })
            .collect::<Vec<_>>()
    }

    let options = if has_options {
        quote! { writer.write_str(" [OPTIONS]")?; }
    } else {
        quote! {}
    };

    quote! {
            writer.write_title("Usage:")?;
            writer.write_str(" ")?;
            parent(writer)?;
            writer.write_str(#name)?;
            #options
            #(#usage_args)*
            writer.writeln_str("")?;
    }
}
