pub use crate::builder::CliBuilder;

use bitflags::bitflags;
use core::fmt::Debug;

#[cfg(not(feature = "history"))]
use core::marker::PhantomData;

use crate::{
    buffer::Buffer,
    codes,
    command::RawCommand,
    editor::Editor,
    input::{ControlInput, Input, InputGenerator},
    service::{Autocomplete, FromRaw, Help, ParseError},
    token::Tokens,
    utils,
    writer::{WriteExt, Writer},
};

#[cfg(feature = "autocomplete")]
use crate::autocomplete::Request;

#[cfg(feature = "help")]
use crate::{help::HelpRequest, service::HelpError};

#[cfg(feature = "history")]
use crate::history::History;

use embedded_io::{Error, Write};

pub struct CliHandle<'a, W: Write<Error = E>, E: embedded_io::Error> {
    dropped_error: &'a mut Option<E>,
    prompt: &'a mut &'static str,
    writer: Writer<'a, W, E>,
}

impl<'a, W, E> CliHandle<'a, W, E>
where
    W: Write<Error = E>,
    E: embedded_io::Error,
{
    /// Set new prompt to use in CLI
    pub fn set_prompt(&mut self, prompt: &'static str) {
        *self.prompt = prompt;
    }

    pub fn writer(&mut self) -> &mut Writer<'a, W, E> {
        &mut self.writer
    }

    fn new(
        dropped_error: &'a mut Option<E>,
        prompt: &'a mut &'static str,
        writer: Writer<'a, W, E>,
    ) -> Self {
        Self {
            dropped_error,
            prompt,
            writer,
        }
    }

    fn cleanup(&mut self) -> Result<(), E> {
        if self.writer.is_dirty() {
            self.writer.write_str(codes::CRLF)?;
        }
        self.writer.write_str(self.prompt)?;
        self.writer.flush()
    }
}

impl<'a, W, E> Debug for CliHandle<'a, W, E>
where
    W: Write<Error = E>,
    E: embedded_io::Error,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CliHandle").finish()
    }
}

impl<'a, W: Write<Error = E>, E: embedded_io::Error> Drop for CliHandle<'a, W, E> {
    fn drop(&mut self) {
        if let Err(err) = self.cleanup() {
            *self.dropped_error = Some(err);
        }
    }
}

#[derive(Debug)]
pub enum CliEvent<'a, C, W: Write<Error = E>, E: embedded_io::Error> {
    Command(C, CliHandle<'a, W, E>),
}

#[cfg(feature = "history")]
enum NavigateHistory {
    Older,
    Newer,
}

enum NavigateInput {
    Backward,
    Forward,
}

bitflags! {
    #[derive(Debug)]
    struct Flags: u8 {
        const EDITOR_CLEANUP_PENDING = 1;
    }
}

#[doc(hidden)]
pub struct Cli<W: Write<Error = E>, E: Error, CommandBuffer: Buffer, HistoryBuffer: Buffer> {
    /// Error that occured while dropping CliHandle
    /// constructed from this Cli.
    /// So we can return it next time user calls cli
    dropped_error: Option<E>,
    editor: Editor<CommandBuffer>,
    flags: Flags,
    #[cfg(feature = "history")]
    history: History<HistoryBuffer>,
    input_generator: InputGenerator,
    prompt: &'static str,
    writer: W,
    #[cfg(not(feature = "history"))]
    _ph: PhantomData<HistoryBuffer>,
}

impl<W, E, CommandBuffer, HistoryBuffer> Debug for Cli<W, E, CommandBuffer, HistoryBuffer>
where
    W: Write<Error = E>,
    E: embedded_io::Error,
    CommandBuffer: Buffer,
    HistoryBuffer: Buffer,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Cli")
            .field("editor", &self.editor)
            .field("input_generator", &self.input_generator)
            .field("prompt", &self.prompt)
            .finish()
    }
}

impl<W, E, CommandBuffer, HistoryBuffer> Cli<W, E, CommandBuffer, HistoryBuffer>
where
    W: Write<Error = E>,
    E: embedded_io::Error,
    CommandBuffer: Buffer,
    HistoryBuffer: Buffer,
{
    pub(crate) fn from_builder(
        builder: CliBuilder<W, E, CommandBuffer, HistoryBuffer>,
    ) -> Result<Cli<W, E, CommandBuffer, HistoryBuffer>, E> {
        let mut cli = Cli {
            dropped_error: None,
            editor: Editor::new(builder.command_buffer),
            flags: Flags::empty(),
            #[cfg(feature = "history")]
            history: History::new(builder.history_buffer),
            input_generator: InputGenerator::new(),
            prompt: builder.prompt,
            writer: builder.writer,
            #[cfg(not(feature = "history"))]
            _ph: PhantomData,
        };

        cli.writer.flush_str(cli.prompt)?;

        Ok(cli)
    }

    /// Each call can be done with different command schema
    pub fn poll<'s: 'e, 'e, C>(&'s mut self, b: u8) -> Result<Option<CliEvent<'e, C, W, E>>, E>
    where
        C: Autocomplete + Help + FromRaw<'e>,
    {
        if let Some(err) = self.dropped_error.take() {
            return Err(err);
        }

        if self.flags.contains(Flags::EDITOR_CLEANUP_PENDING) {
            self.flags.set(Flags::EDITOR_CLEANUP_PENDING, false);
            self.editor.clear();
        }

        if let Some(input) = self.input_generator.accept(b) {
            match input {
                Input::Control(control) => return self.on_control_input::<C>(control),
                Input::Char(text) => {
                    let is_inside = self.editor.cursor() < self.editor.len();
                    if let Some(c) = self.editor.insert(text) {
                        if is_inside {
                            // text is always one char
                            debug_assert_eq!(c.chars().count(), 1);
                            self.writer.write_bytes(codes::INSERT_CHAR)?;
                        }
                        self.writer.flush_str(c)?;
                    }
                }
            }
        }

        Ok(None)
    }

    /// Set new prompt to use in CLI
    ///
    /// Changes will apply immediately and current line
    /// will be replaced by new prompt and input
    pub fn set_prompt(&mut self, prompt: &'static str) -> Result<(), E> {
        self.prompt = prompt;
        self.clear_line(false)?;

        self.writer.flush_str(self.editor.text())?;

        Ok(())
    }

    pub fn write(
        &mut self,
        f: impl FnOnce(&mut Writer<'_, W, E>) -> Result<(), E>,
    ) -> Result<(), E> {
        self.clear_line(true)?;

        let mut cli_writer = Writer::new(&mut self.writer);

        f(&mut cli_writer)?;

        // we should write back input that was there before writing
        if cli_writer.is_dirty() {
            self.writer.write_str(codes::CRLF)?;
        }
        self.writer.write_str(self.prompt)?;
        self.writer.flush_str(self.editor.text())?;

        Ok(())
    }

    fn clear_line(&mut self, clear_prompt: bool) -> Result<(), E> {
        self.writer.write_str("\r")?;
        self.writer.write_bytes(codes::CLEAR_LINE)?;

        if !clear_prompt {
            self.writer.write_str(self.prompt)?;
        }

        self.writer.flush()
    }

    fn on_control_input<'s: 'e, 'e, C>(
        &'s mut self,
        control: ControlInput,
    ) -> Result<Option<CliEvent<'e, C, W, E>>, E>
    where
        C: Autocomplete + Help + FromRaw<'e>,
    {
        match control {
            ControlInput::Enter => {
                self.flags.set(Flags::EDITOR_CLEANUP_PENDING, true);
                self.writer.write_str(codes::CRLF)?;

                #[cfg(feature = "history")]
                self.history.push(self.editor.text());
                return self.process_input::<C>();
            }
            ControlInput::Tab => {
                #[cfg(feature = "autocomplete")]
                self.process_autocomplete::<C>()?;
            }
            ControlInput::Backspace => {
                if self.editor.move_left() {
                    self.editor.remove();
                    self.writer.flush_bytes(codes::CURSOR_BACKWARD)?;
                    self.writer.flush_bytes(codes::DELETE_CHAR)?;
                }
            }
            ControlInput::Down =>
            {
                #[cfg(feature = "history")]
                self.navigate_history(NavigateHistory::Newer)?
            }
            ControlInput::Up =>
            {
                #[cfg(feature = "history")]
                self.navigate_history(NavigateHistory::Older)?
            }
            ControlInput::Forward => self.navigate_input(NavigateInput::Forward)?,
            ControlInput::Back => self.navigate_input(NavigateInput::Backward)?,
        }

        Ok(None)
    }

    fn navigate_input(&mut self, dir: NavigateInput) -> Result<(), E> {
        match dir {
            NavigateInput::Backward if self.editor.move_left() => {
                self.writer.flush_bytes(codes::CURSOR_BACKWARD)?;
            }
            NavigateInput::Forward if self.editor.move_right() => {
                self.writer.flush_bytes(codes::CURSOR_FORWARD)?;
            }
            _ => return Ok(()),
        }
        Ok(())
    }

    #[cfg(feature = "history")]
    fn navigate_history(&mut self, dir: NavigateHistory) -> Result<(), E> {
        let history_elem = match dir {
            NavigateHistory::Older => self.history.next_older(),
            NavigateHistory::Newer => self.history.next_newer().or(Some("")),
        };
        if let Some(element) = history_elem {
            self.editor.clear();
            self.editor.insert(element);
            self.clear_line(false)?;

            self.writer.flush_str(self.editor.text())?;
        }
        Ok(())
    }

    #[cfg(feature = "autocomplete")]
    fn process_autocomplete<C: Autocomplete>(&mut self) -> Result<(), E> {
        let initial_cursor = self.editor.cursor();
        self.editor.autocompletion(|request, autocompletion| {
            C::autocomplete(request.clone(), autocompletion);
            match request {
                Request::CommandName(name) if "help".starts_with(name) => {
                    // SAFETY: "help" starts with name, so name cannot be longer
                    let autocompleted = unsafe { "help".get_unchecked(name.len()..) };
                    autocompletion.merge_autocompletion(autocompleted)
                }
                _ => {}
            }
        });
        if self.editor.cursor() > initial_cursor {
            let autocompleted = self.editor.text_range(initial_cursor..);
            self.writer.flush_str(autocompleted)?;
        }
        Ok(())
    }

    fn process_input<'s: 'e, 'e, C>(&'s mut self) -> Result<Option<CliEvent<'e, C, W, E>>, E>
    where
        C: Help + FromRaw<'e>,
    {
        let text = self.editor.text_mut();

        let tokens = Tokens::new(text);
        if let Some(command) = RawCommand::from_tokens(&tokens) {
            #[cfg(feature = "help")]
            if let Some(request) = HelpRequest::from_command(&command) {
                Self::process_help::<C>(&mut self.writer, request)?;
                self.writer.flush_str(self.prompt)?;
                return Ok(None);
            }

            match C::parse(command) {
                Err(err) => {
                    Self::process_error(&mut self.writer, err)?;
                }
                Ok(cmd) => {
                    let cli_writer = Writer::new(&mut self.writer);
                    let handle =
                        CliHandle::new(&mut self.dropped_error, &mut self.prompt, cli_writer);
                    return Ok(Some(CliEvent::Command(cmd, handle)));
                }
            }
        }
        self.writer.flush_str(self.prompt)?;

        Ok(None)
    }

    fn process_error(writer: &mut W, error: ParseError<'_>) -> Result<(), E> {
        writer.write_str("error: ")?;
        match error {
            ParseError::MissingRequiredArgument { name } => {
                writer.write_str("missing required argument: ")?;
                writer.write_str(name)?;
            }
            ParseError::ParseValueError { value, expected } => {
                writer.write_str("failed to parse '")?;
                writer.write_str(value)?;
                writer.write_str("', expected ")?;
                writer.write_str(expected)?;
            }
            ParseError::UnexpectedArgument { value } => {
                writer.write_str("unexpected argument: ")?;
                writer.write_str(value)?;
            }
            ParseError::UnexpectedLongOption { name } => {
                writer.write_str("unexpected option: -")?;
                writer.write_str("-")?;
                writer.write_str(name)?;
            }
            ParseError::UnexpectedShortOption { name } => {
                let mut buf = [0; 4];
                let buf = utils::encode_utf8(name, &mut buf);
                writer.write_str("unexpected option: -")?;
                writer.write_str(buf)?;
            }
            ParseError::UnknownCommand => {
                writer.write_str("unknown command")?;
            }
        }
        writer.write_str(codes::CRLF)
    }

    #[cfg(feature = "help")]
    fn process_help<C: Help>(writer: &mut W, request: HelpRequest<'_>) -> Result<(), E> {
        let mut writer_wrapper = Writer::new(writer);

        match request {
            HelpRequest::All => C::list_commands(&mut writer_wrapper)?,
            HelpRequest::Command(command) => {
                match C::command_help(&mut |_| Ok(()), command.clone(), &mut writer_wrapper) {
                    Err(HelpError::UnknownCommand) => {
                        writer_wrapper.write_str("error: ")?;
                        writer_wrapper.write_str("unknown command")?;
                    }
                    Err(HelpError::WriteError(err)) => return Err(err),
                    Ok(()) => {}
                }
            }
        };

        if writer_wrapper.is_dirty() {
            writer.write_str(codes::CRLF)?;
        }

        Ok(())
    }
}
