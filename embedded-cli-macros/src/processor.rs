use darling::Result;
use proc_macro2::TokenStream;
use quote::quote;

use crate::utils::TargetType;

pub fn impl_processor(target: &TargetType) -> Result<TokenStream> {
    let ident = target.ident();
    let named_lifetime = target.named_lifetime();
    let unnamed_lifetime = target.unnamed_lifetime();

    let output = quote! {

        impl #named_lifetime #ident #named_lifetime {
            fn processor<
                W: _io::Write<Error = E>,
                E: _io::Error,
                F: FnMut(&mut _cli::cli::CliHandle<'_, W, E>, #ident #unnamed_lifetime) -> Result<(), E>,
            >(
                f: F,
            ) -> impl _cli::service::CommandProcessor<W, E> {
                struct Processor<
                    W: _io::Write<Error = E>,
                    E: _io::Error,
                    F: FnMut(&mut _cli::cli::CliHandle<'_, W, E>, #ident #unnamed_lifetime) -> Result<(), E>,
                > {
                    f: F,
                    _ph: core::marker::PhantomData<(W, E)>,
                }

                impl<
                        W: _io::Write<Error = E>,
                        E: _io::Error,
                        F: FnMut(&mut _cli::cli::CliHandle<'_, W, E>, #ident #unnamed_lifetime) -> Result<(), E>,
                    > _cli::service::CommandProcessor<W, E> for Processor<W, E, F>
                {
                    fn process<'a>(
                        &mut self,
                        cli: &mut _cli::cli::CliHandle<'_, W, E>,
                        raw: _cli::command::RawCommand<'a>,
                    ) -> Result<(), _cli::service::ProcessError<'a, E>> {
                        let cmd = <#ident as _cli::service::FromRaw>::parse(raw)?;
                        (self.f)(cli, cmd)?;
                        Ok(())
                    }
                }

                Processor {
                    f,
                    _ph: core::marker::PhantomData,
                }
            }
        }
    };

    Ok(output)
}
