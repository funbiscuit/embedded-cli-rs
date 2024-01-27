use darling::Result;
use proc_macro2::TokenStream;
use quote::quote;

use super::{model::Command, TargetType};

#[cfg(feature = "help")]
use super::model::{CommandArgType, CommandArgs};

#[cfg(feature = "help")]
pub fn derive_help(
    target: &TargetType,
    help_title: &str,
    commands: &[Command],
) -> Result<TokenStream> {
    let help_all = create_help_all(commands, help_title)?;
    let commands_help = commands.iter().map(create_command_help).collect::<Vec<_>>();

    let ident = target.ident();
    let named_lifetime = target.named_lifetime();

    let output = quote! {
        impl #named_lifetime _cli::service::Help for #ident #named_lifetime {
            fn help<W: _io::Write<Error = E>, E: _io::Error>(
                request: _cli::help::HelpRequest<'_>,
                writer: &mut _cli::writer::Writer<'_, W, E>,
            ) -> Result<(), _cli::service::HelpError<E>> {
                match request {
                    _cli::help::HelpRequest::All => {
                        #help_all
                        Ok(())
                    }
                    _cli::help::HelpRequest::Command(command) => {
                        match command {
                            #(#commands_help)*
                            _ => return Err(_cli::service::HelpError::UnknownCommand),
                        }

                        Ok(())
                    }
                }
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
        impl #named_lifetime _cli::service::Help for #ident #named_lifetime { }
    };

    Ok(output)
}

#[cfg(feature = "help")]
fn create_help_all(commands: &[Command], title: &str) -> Result<TokenStream> {
    let max_len = commands.iter().map(|c| c.name().len()).max().unwrap_or(0);
    let elements: Vec<_> = commands
        .iter()
        .map(|c| {
            let name = c.name();
            let help = c.help().short().unwrap_or("");
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
        writer.writeln_str("")?;
    })
}

#[cfg(feature = "help")]
fn create_command_help(command: &Command) -> TokenStream {
    let name = command.name();

    let help = if let Some(help) = command.help().long() {
        quote! {
            writer.writeln_str(#help)?;
            writer.writeln_str("")?;
        }
    } else {
        quote! {}
    };

    let usage = create_usage(command.args());
    let args_help = create_args_help(command.args());
    let options_help = create_options_help(command.args());

    quote! {
        #name => {
            #help
            #usage
            #args_help
            #options_help
        },
    }
}

#[cfg(feature = "help")]
fn create_args_help(args: &CommandArgs) -> TokenStream {
    let help_lines = match args {
        CommandArgs::None => vec![],
        CommandArgs::Named(args) => {
            // 2 is added to account for brackets
            let longest_arg = args
                .iter()
                .filter(|a| a.arg_type().is_positional())
                .map(|a| a.name().len() + 2)
                .max()
                .unwrap_or(0);

            args.iter()
                .filter_map(|arg| match arg.arg_type() {
                    CommandArgType::Positional => {
                        let name = if arg.is_optional() {
                            format!("[{}]", arg.name().to_uppercase())
                        } else {
                            format!("<{}>", arg.name().to_uppercase())
                        };

                        let arg_help = arg.help().short().unwrap_or("");

                        Some(quote! {
                            writer.write_list_element(#name, #arg_help, #longest_arg)?;
                        })
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        }
    };

    if help_lines.is_empty() {
        quote! {}
    } else {
        quote! {
           writer.write_title("Arguments:")?;
           writer.writeln_str("")?;
           #(#help_lines)*
           writer.writeln_str("")?;
        }
    }
}

#[cfg(feature = "help")]
fn create_options_help(args: &CommandArgs) -> TokenStream {
    struct OptionHelp {
        name: String,
        help: String,
    }

    let mut help_lines = match args {
        CommandArgs::None => vec![],
        CommandArgs::Named(args) => args
            .iter()
            .filter_map(|arg| match arg.arg_type() {
                CommandArgType::Flag { long, short } => {
                    let name = short
                        .map(|name| format!("-{}", name))
                        .into_iter()
                        .chain(long.iter().map(|name| format!("--{}", name)))
                        .collect::<Vec<_>>()
                        .join(", ");

                    let help = arg.help().short().unwrap_or("").to_string();

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
                        format!("[{}]", arg.name().to_uppercase())
                    } else {
                        format!("<{}>", arg.name().to_uppercase())
                    };

                    let name = format!("{} {}", name, value);

                    let help = arg.help().short().unwrap_or("").to_string();

                    Some(OptionHelp { name, help })
                }
                CommandArgType::Positional => None,
            })
            .collect::<Vec<_>>(),
    };
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
fn create_usage(args: &CommandArgs) -> TokenStream {
    let has_options;
    let usage_args;

    match args {
        CommandArgs::None => {
            has_options = false;
            usage_args = vec![quote! { writer.writeln_str("")?; }];
        }
        CommandArgs::Named(args) => {
            has_options = args.iter().any(|arg| !arg.arg_type().is_positional());

            usage_args = args
                .iter()
                .filter_map(|arg| match arg.arg_type() {
                    crate::command::model::CommandArgType::Positional => {
                        let name = if arg.is_optional() {
                            format!("[{}]", arg.name().to_uppercase())
                        } else {
                            format!("<{}>", arg.name().to_uppercase())
                        };
                        Some(name)
                    }
                    _ => None,
                })
                .map(|line| {
                    quote! {
                        writer.write_str(" ")?;
                        writer.writeln_str(#line)?;
                    }
                })
                .collect::<Vec<_>>()
        }
    };

    let options = if has_options {
        quote! { writer.write_str(" [OPTIONS]")?; }
    } else {
        quote! {}
    };

    quote! {
            writer.write_title("Usage:")?;
            writer.write_str(" ")?;
            writer.write_str(command)?;
            #options
            #(#usage_args)*
            writer.writeln_str("")?;
    }
}
