pub use crate::builder::CliBuilder;

use core::fmt::Debug;

use crate::{
    autocomplete::Request,
    buffer::Buffer,
    codes,
    command::RawCommand,
    editor::Editor,
    help::HelpRequest,
    history::History,
    service::{Autocomplete, CommandProcessor, Help, HelpError, ProcessError},
    token::Tokens,
    utf8::Utf8Accum,
    writer::{WriteExt, Writer},
};

use bitflags::bitflags;
use embedded_io::{Error, Write};

const PROMPT: &str = "$ ";

bitflags! {
    #[derive(Debug)]
    struct CliFlags: u8 {
        const ESCAPE_MODE = 1;
    }
}

pub struct CliHandle<'a, W: Write<Error = E>, E: embedded_io::Error> {
    writer: Writer<'a, W, E>,
}

impl<'a, W, E> CliHandle<'a, W, E>
where
    W: Write<Error = E>,
    E: embedded_io::Error,
{
    pub fn writer(&mut self) -> &mut Writer<'a, W, E> {
        &mut self.writer
    }

    fn new(writer: Writer<'a, W, E>) -> Self {
        Self { writer }
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

#[doc(hidden)]
pub struct Cli<W: Write<Error = E>, E: Error, CommandBuffer: Buffer, HistoryBuffer: Buffer> {
    char_accum: Utf8Accum,
    editor: Option<Editor<CommandBuffer>>,
    history: History<HistoryBuffer>,
    prompt: &'static str,
    last_control: Option<ControlInput>,
    flags: CliFlags,
    writer: W,
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
            .field("char_accum", &self.char_accum)
            .field("editor", &self.editor)
            .field("prompt", &self.prompt)
            .field("last_control", &self.last_control)
            .field("flags", &self.flags)
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
    pub fn new(
        writer: W,
        command_buffer: CommandBuffer,
        history_buffer: HistoryBuffer,
    ) -> Result<Self, E> {
        let history: History<HistoryBuffer> = History::new(history_buffer);

        let mut cli = Self {
            char_accum: Utf8Accum::default(),
            editor: Some(Editor::new(command_buffer)),
            history,
            prompt: PROMPT,
            last_control: None,
            flags: CliFlags::empty(),
            writer,
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
        if let Some(mut editor) = self.editor.take() {
            if self.flags.contains(CliFlags::ESCAPE_MODE) {
                self.on_escaped_input(&mut editor, b)?;
            } else if self.last_control == Some(ControlInput::Escape) && b == b'[' {
                self.flags.set(CliFlags::ESCAPE_MODE, true);
            } else {
                match Input::parse(b) {
                    Some(Input::ControlInput(input)) => {
                        self.on_control_input::<C, _>(&mut editor, input, processor)?
                    }
                    Some(Input::Other(input)) => self.on_char_input(&mut editor, input)?,
                    _ => {}
                }
            }

            self.editor = Some(editor);
        }
        Ok(())
    }

    pub fn write(
        &mut self,
        f: impl FnOnce(&mut Writer<'_, W, E>) -> Result<(), E>,
    ) -> Result<(), E> {
        if let Some(input_len) = self.editor.as_ref().map(|e| e.len()) {
            self.clear_line(input_len, true)?;
        }

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

    fn clear_line(&mut self, input_len: usize, clear_prompt: bool) -> Result<(), E> {
        self.writer.write_str("\r")?;

        if clear_prompt {
            for _ in 0..self.prompt.len() {
                self.writer.write_str(" ")?;
            }
        } else {
            self.writer.write_str(self.prompt)?;
        }

        for _ in 0..input_len {
            self.writer.write_str(" ")?;
        }

        if clear_prompt {
            self.writer.write_str("\r")?;
        } else {
            self.writer.write_str("\r")?;
            self.writer.write_str(self.prompt)?;
        }
        self.writer.flush()
    }

    fn on_char_input(&mut self, editor: &mut Editor<CommandBuffer>, input: u8) -> Result<(), E> {
        if let Some(c) = self.char_accum.push_byte(input) {
            if let Some(c) = editor.insert(c) {
                //TODO: cursor position not at end
                self.writer.flush_str(c)?;
            }
        }
        Ok(())
    }

    fn on_control_input<C: Autocomplete + Help, P: CommandProcessor<W, E>>(
        &mut self,
        editor: &mut Editor<CommandBuffer>,
        input: ControlInput,
        processor: &mut P,
    ) -> Result<(), E> {
        // handle \r\n and \n\r as single \n
        if (self.last_control == Some(ControlInput::CarriageReturn)
            && input == ControlInput::LineFeed)
            || (self.last_control == Some(ControlInput::LineFeed)
                && input == ControlInput::CarriageReturn)
        {
            self.last_control = None;
            return Ok(());
        }

        match input {
            ControlInput::CarriageReturn | ControlInput::LineFeed => {
                self.writer.write_str(codes::CRLF)?;

                let text = editor.text();
                self.history.push(text);
                let text = editor.text_mut();

                if let Some(tokens) = Tokens::new(text) {
                    self.process_input::<C, _>(tokens, processor)?;
                }

                editor.clear();

                self.writer.flush_str(self.prompt)?;
            }
            ControlInput::Escape => {}
            ControlInput::Tabulation => {
                self.process_autocomplete::<C>(editor)?;
            }
            ControlInput::Backspace => {
                if editor.move_left() {
                    editor.remove();
                    self.writer.flush_str("\x08 \x08")?;
                }
            }
        }

        self.last_control = Some(input);
        Ok(())
    }

    fn on_escaped_input(&mut self, editor: &mut Editor<CommandBuffer>, input: u8) -> Result<(), E> {
        if (0x40..=0x7E).contains(&input) {
            // handle escape sequence
            self.flags.remove(CliFlags::ESCAPE_MODE);

            // treat \e[..A as cursor up and \e[..B as cursor down
            //TODO: there might be extra chars between \e[ and A/B, probably should not ignore them
            let history_elem = match input {
                b'A' => self.history.next_older(),
                b'B' => self.history.next_newer().or(Some("")),
                _ => None,
            };
            if let Some(element) = history_elem {
                let input_len = editor.len();
                editor.clear();
                editor.insert(element);
                self.clear_line(input_len, false)?;

                self.writer.flush_str(editor.text())?;
            }
        }
        Ok(())
    }

    fn process_autocomplete<C: Autocomplete>(
        &mut self,
        editor: &mut Editor<CommandBuffer>,
    ) -> Result<(), E> {
        let initial_cursor = editor.cursor();
        editor.autocompletion(|request, autocompletion| {
            C::autocomplete(request.clone(), autocompletion);
            match request {
                Request::CommandName(name) if "help".starts_with(name) => {
                    autocompletion.merge_autocompletion(&"help"[name.len()..])
                }
                _ => {}
            }
        });
        let autocompleted = editor.text_range(initial_cursor..);
        if !autocompleted.is_empty() {
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

        if handle.writer.is_dirty() {
            self.writer.write_str(codes::CRLF)?;
        }
        self.writer.flush()?;

        if let Err(e) = res {
            self.process_error(e)?;
        }

        Ok(())
    }

    fn process_input<C: Autocomplete + Help, P: CommandProcessor<W, E>>(
        &mut self,
        tokens: Tokens<'_>,
        handler: &mut P,
    ) -> Result<(), E> {
        match HelpRequest::from_tokens(tokens) {
            Ok(request) => {
                self.process_help::<C>(request)?;
            }
            Err(tokens) => {
                if let Some(command) = RawCommand::from_tokens(tokens) {
                    self.process_command(command, handler)?;
                };
            }
        }

        Ok(())
    }

    fn process_error(&mut self, _error: ProcessError<'_, E>) -> Result<(), E> {
        //TODO: proper handling
        self.writer
            .flush_str("Error occured during command processing\r\n")
    }

    fn process_help<C: Help>(&mut self, request: HelpRequest<'_>) -> Result<(), E> {
        let mut writer = Writer::new(&mut self.writer);
        let err = C::help(request.clone(), &mut writer);

        if let (Err(HelpError::UnknownCommand), HelpRequest::Command(command)) = (err, request) {
            writer.write_str("error: unrecognized command '")?;
            writer.write_str(command)?;
            writer.write_str("'")?;
        }

        if writer.is_dirty() {
            self.writer.write_str(codes::CRLF)?;
        }
        self.writer.flush()?;

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ControlInput {
    Backspace,
    CarriageReturn,
    Escape,
    LineFeed,
    Tabulation,
}

#[derive(Debug)]
enum Input {
    ControlInput(ControlInput),
    Other(u8),
}

impl Input {
    pub fn parse(byte: u8) -> Option<Input> {
        let input = match byte {
            codes::BACKSPACE => Input::ControlInput(ControlInput::Backspace),
            codes::CARRIAGE_RETURN => Input::ControlInput(ControlInput::CarriageReturn),
            codes::ESCAPE => Input::ControlInput(ControlInput::Escape),
            codes::LINE_FEED => Input::ControlInput(ControlInput::LineFeed),
            codes::TABULATION => Input::ControlInput(ControlInput::Tabulation),
            b => Input::Other(b),
        };
        Some(input)
    }
}
