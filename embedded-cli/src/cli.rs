pub use crate::builder::CliBuilder;

use core::fmt::Debug;

#[cfg(not(feature = "history"))]
use core::marker::PhantomData;

use crate::{
    buffer::Buffer,
    builder::DEFAULT_PROMPT,
    codes,
    command::RawCommand,
    editor::Editor,
    input::{ControlInput, Input, InputGenerator},
    service::{Autocomplete, CommandProcessor, Help, ParseError, ProcessError},
    token::Tokens,
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
    new_prompt: Option<&'static str>,
    writer: Writer<'a, W, E>,
}

impl<'a, W, E> CliHandle<'a, W, E>
where
    W: Write<Error = E>,
    E: embedded_io::Error,
{
    /// Set new prompt to use in CLI
    pub fn set_prompt(&mut self, prompt: &'static str) {
        self.new_prompt = Some(prompt)
    }

    pub fn writer(&mut self) -> &mut Writer<'a, W, E> {
        &mut self.writer
    }

    fn new(writer: Writer<'a, W, E>) -> Self {
        Self {
            new_prompt: None,
            writer,
        }
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

#[cfg(feature = "history")]
enum NavigateHistory {
    Older,
    Newer,
}

enum NavigateInput {
    Backward,
    Forward,
}

#[doc(hidden)]
pub struct Cli<W: Write<Error = E>, E: Error, CommandBuffer: Buffer, HistoryBuffer: Buffer> {
    editor: Option<Editor<CommandBuffer>>,
    #[cfg(feature = "history")]
    history: History<HistoryBuffer>,
    input_generator: Option<InputGenerator>,
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
    #[allow(unused_variables)]
    #[deprecated(since = "0.2.1", note = "please use `builder` instead")]
    pub fn new(
        writer: W,
        command_buffer: CommandBuffer,
        history_buffer: HistoryBuffer,
    ) -> Result<Self, E> {
        let mut cli = Self {
            editor: Some(Editor::new(command_buffer)),
            #[cfg(feature = "history")]
            history: History::new(history_buffer),
            input_generator: Some(InputGenerator::new()),
            prompt: DEFAULT_PROMPT,
            writer,
            #[cfg(not(feature = "history"))]
            _ph: PhantomData,
        };

        cli.writer.flush_str(cli.prompt)?;

        Ok(cli)
    }

    pub(crate) fn from_builder(
        builder: CliBuilder<W, E, CommandBuffer, HistoryBuffer>,
    ) -> Result<Self, E> {
        let mut cli = Self {
            editor: Some(Editor::new(builder.command_buffer)),
            #[cfg(feature = "history")]
            history: History::new(builder.history_buffer),
            input_generator: Some(InputGenerator::new()),
            prompt: builder.prompt,
            writer: builder.writer,
            #[cfg(not(feature = "history"))]
            _ph: PhantomData,
        };

        cli.writer.flush_str(cli.prompt)?;

        Ok(cli)
    }

    /// Each call to process byte can be done with different
    /// command set and/or command processor.
    /// In process callback you can change some outside state
    /// so next calls will use different processor
    pub fn process_byte<C: Autocomplete + Help, P: CommandProcessor<W, E>>(
        &mut self,
        b: u8,
        processor: &mut P,
    ) -> Result<(), E> {
        if let (Some(mut editor), Some(mut input_generator)) =
            (self.editor.take(), self.input_generator.take())
        {
            let result = input_generator
                .accept(b)
                .map(|input| match input {
                    Input::Control(control) => {
                        self.on_control_input::<C, _>(&mut editor, control, processor)
                    }
                    Input::Char(text) => self.on_text_input(&mut editor, text),
                })
                .unwrap_or(Ok(()));

            self.editor = Some(editor);
            self.input_generator = Some(input_generator);
            result
        } else {
            Ok(())
        }
    }

    /// Set new prompt to use in CLI
    ///
    /// Changes will apply immediately and current line
    /// will be replaced by new prompt and input
    pub fn set_prompt(&mut self, prompt: &'static str) -> Result<(), E> {
        self.prompt = prompt;
        self.clear_line(false)?;

        if let Some(editor) = self.editor.as_mut() {
            self.writer.flush_str(editor.text())?;
        }

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
        if let Some(editor) = self.editor.as_mut() {
            self.writer.flush_str(editor.text())?;
        }

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

    fn on_text_input(&mut self, editor: &mut Editor<CommandBuffer>, text: &str) -> Result<(), E> {
        let is_inside = editor.cursor() < editor.len();
        if let Some(c) = editor.insert(text) {
            if is_inside {
                // text is always one char
                debug_assert_eq!(c.chars().count(), 1);
                self.writer.write_bytes(codes::INSERT_CHAR)?;
            }
            self.writer.flush_str(c)?;
        }
        Ok(())
    }

    fn on_control_input<C: Autocomplete + Help, P: CommandProcessor<W, E>>(
        &mut self,
        editor: &mut Editor<CommandBuffer>,
        control: ControlInput,
        processor: &mut P,
    ) -> Result<(), E> {
        match control {
            ControlInput::Enter => {
                self.writer.write_str(codes::CRLF)?;

                #[cfg(feature = "history")]
                self.history.push(editor.text());
                let text = editor.text_mut();

                let tokens = Tokens::new(text);
                self.process_input::<C, _>(tokens, processor)?;

                editor.clear();

                self.writer.flush_str(self.prompt)?;
            }
            ControlInput::Tab => {
                #[cfg(feature = "autocomplete")]
                self.process_autocomplete::<C>(editor)?;
            }
            ControlInput::Backspace => {
                if editor.move_left() {
                    editor.remove();
                    self.writer.flush_bytes(codes::CURSOR_BACKWARD)?;
                    self.writer.flush_bytes(codes::DELETE_CHAR)?;
                }
            }
            ControlInput::Down =>
            {
                #[cfg(feature = "history")]
                self.navigate_history(editor, NavigateHistory::Newer)?
            }
            ControlInput::Up =>
            {
                #[cfg(feature = "history")]
                self.navigate_history(editor, NavigateHistory::Older)?
            }
            ControlInput::Forward => self.navigate_input(editor, NavigateInput::Forward)?,
            ControlInput::Back => self.navigate_input(editor, NavigateInput::Backward)?,
        }

        Ok(())
    }

    fn navigate_input(
        &mut self,
        editor: &mut Editor<CommandBuffer>,
        dir: NavigateInput,
    ) -> Result<(), E> {
        match dir {
            NavigateInput::Backward if editor.move_left() => {
                self.writer.flush_bytes(codes::CURSOR_BACKWARD)?;
            }
            NavigateInput::Forward if editor.move_right() => {
                self.writer.flush_bytes(codes::CURSOR_FORWARD)?;
            }
            _ => return Ok(()),
        }
        Ok(())
    }

    #[cfg(feature = "history")]
    fn navigate_history(
        &mut self,
        editor: &mut Editor<CommandBuffer>,
        dir: NavigateHistory,
    ) -> Result<(), E> {
        let history_elem = match dir {
            NavigateHistory::Older => self.history.next_older(),
            NavigateHistory::Newer => self.history.next_newer().or(Some("")),
        };
        if let Some(element) = history_elem {
            editor.clear();
            editor.insert(element);
            self.clear_line(false)?;

            self.writer.flush_str(editor.text())?;
        }
        Ok(())
    }

    #[cfg(feature = "autocomplete")]
    fn process_autocomplete<C: Autocomplete>(
        &mut self,
        editor: &mut Editor<CommandBuffer>,
    ) -> Result<(), E> {
        let initial_cursor = editor.cursor();
        editor.autocompletion(|request, autocompletion| {
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
        if editor.cursor() > initial_cursor {
            let autocompleted = editor.text_range(initial_cursor..);
            self.writer.flush_str(autocompleted)?;
        }
        Ok(())
    }

    fn process_command<P: CommandProcessor<W, E>>(
        &mut self,
        command: RawCommand<'_>,
        handler: &mut P,
    ) -> Result<(), E> {
        let cli_writer = Writer::new(&mut self.writer);
        let mut handle = CliHandle::new(cli_writer);

        let res = handler.process(&mut handle, command);

        if let Some(prompt) = handle.new_prompt {
            self.prompt = prompt;
        }
        if handle.writer.is_dirty() {
            self.writer.write_str(codes::CRLF)?;
        }
        self.writer.flush()?;

        match res {
            Err(ProcessError::ParseError(err)) => self.process_error(err),
            Err(ProcessError::WriteError(err)) => Err(err),
            Ok(()) => Ok(()),
        }
    }

    #[allow(clippy::extra_unused_type_parameters)]
    fn process_input<C: Help, P: CommandProcessor<W, E>>(
        &mut self,
        tokens: Tokens<'_>,
        handler: &mut P,
    ) -> Result<(), E> {
        if let Some(command) = RawCommand::from_tokens(&tokens) {
            #[cfg(feature = "help")]
            if let Some(request) = HelpRequest::from_command(&command) {
                return self.process_help::<C>(request);
            }

            self.process_command(command, handler)?;
        };

        Ok(())
    }

    fn process_error(&mut self, error: ParseError<'_>) -> Result<(), E> {
        self.writer.write_str("error: ")?;
        match error {
            ParseError::MissingRequiredArgument { name } => {
                self.writer.write_str("missing required argument: ")?;
                self.writer.write_str(name)?;
            }
            ParseError::NonAsciiShortOption => {
                self.writer
                    .write_str("non-ascii in short options is not supported")?;
            }
            ParseError::ParseValueError { value, expected } => {
                self.writer.write_str("failed to parse '")?;
                self.writer.write_str(value)?;
                self.writer.write_str("', expected ")?;
                self.writer.write_str(expected)?;
            }
            ParseError::UnexpectedArgument { value } => {
                self.writer.write_str("unexpected argument: ")?;
                self.writer.write_str(value)?;
            }
            ParseError::UnexpectedLongOption { name } => {
                self.writer.write_str("unexpected option: -")?;
                self.writer.write_str("-")?;
                self.writer.write_str(name)?;
            }
            ParseError::UnexpectedShortOption { name } => {
                // short options are guaranteed to be ascii alphabetic
                if name.is_ascii_alphabetic() {
                    self.writer.write_str("unexpected option: -")?;
                    self.writer.write_bytes(&[name as u8])?;
                }
            }
            ParseError::UnknownCommand => {
                self.writer.write_str("unknown command")?;
            }
        }
        self.writer.flush_str(codes::CRLF)
    }

    #[cfg(feature = "help")]
    fn process_help<C: Help>(&mut self, request: HelpRequest<'_>) -> Result<(), E> {
        let mut writer = Writer::new(&mut self.writer);

        match request {
            HelpRequest::All => C::list_commands(&mut writer)?,
            HelpRequest::Command(command) => {
                match C::command_help(&mut |_| Ok(()), command.clone(), &mut writer) {
                    Err(HelpError::UnknownCommand) => {
                        writer.write_str("error: ")?;
                        writer.write_str("unknown command")?;
                    }
                    Err(HelpError::WriteError(err)) => return Err(err),
                    Ok(()) => {}
                }
            }
        };

        if writer.is_dirty() {
            self.writer.write_str(codes::CRLF)?;
        }
        self.writer.flush()?;

        Ok(())
    }
}
