use darling::Result;
use proc_macro2::TokenStream;
use quote::quote;

use super::{model::Command, TargetType};

#[cfg(feature = "help")]
use super::model::CommandArgs;

#[cfg(feature = "help")]
pub fn derive_help(
    target: &TargetType,
    help_title: &str,
    commands: &[Command],
) -> Result<TokenStream> {
    let help_all = create_help_all(commands, help_title)?;
    let commands_help = commands
        .iter()
        .map(create_command_help)
        .collect::<Result<Vec<_>>>()?;

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
fn create_command_help(command: &Command) -> Result<TokenStream> {
    let name = command.name();

    let help = if let Some(help) = command.help().long() {
        quote! {
            writer.writeln_str(#help)?;
            writer.writeln_str("")?;
        }
    } else {
        quote! {}
    };

    let (args_str, args_help) = match command.args() {
        CommandArgs::None => (quote! { "" }, quote! {}),
        CommandArgs::Named(args) => {
            let args_str = args
                .iter()
                .map(|arg| {
                    if arg.is_optional() {
                        format!(" [{}]", arg.name().to_uppercase())
                    } else {
                        format!(" <{}>", arg.name().to_uppercase())
                    }
                })
                .collect::<String>();
            let longest_arg = args.iter().map(|a| a.name().len() + 2).max().unwrap_or(0);

            let args_help = args
                .iter()
                .map(|arg| {
                    let name = if arg.is_optional() {
                        format!("[{}]", arg.name().to_uppercase())
                    } else {
                        format!("<{}>", arg.name().to_uppercase())
                    };

                    let arg_help = arg.help().short().unwrap_or("");

                    quote! {
                        writer.write_list_element(#name, #arg_help, #longest_arg)?;
                    }
                })
                .collect::<Vec<_>>();

            let args_str = quote! { #args_str };
            let args_help = if args_help.is_empty() {
                quote! {}
            } else {
                quote! {
                   writer.write_title("Arguments:")?;
                   writer.writeln_str("")?;
                   #(#args_help)*
                   writer.writeln_str("")?;
                }
            };

            (args_str, args_help)
        }
    };

    Ok(quote! {
        #name => {
            #help

            writer.write_title("Usage:")?;
            writer.write_str(" ")?;
            writer.write_str(command)?;
            writer.writeln_str(#args_str)?;
            writer.writeln_str("")?;

            #args_help

            writer.write_title("Options:")?;
            writer.writeln_str("")?;
            writer.write_list_element("-h, --help", "Print help", 10)?;
        },
    })
}
