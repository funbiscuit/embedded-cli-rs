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
    input::{ControlInput, Input, InputGenerator},
    service::{Autocomplete, CommandProcessor, Help, HelpError, ProcessError},
    token::Tokens,
    writer::{WriteExt, Writer},
};

use embedded_io::{Error, Write};

const PROMPT: &str = "$ ";

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

enum NavigateHistory {
    Older,
    Newer,
}

#[doc(hidden)]
pub struct Cli<W: Write<Error = E>, E: Error, CommandBuffer: Buffer, HistoryBuffer: Buffer> {
    editor: Option<Editor<CommandBuffer>>,
    history: History<HistoryBuffer>,
    input_generator: Option<InputGenerator>,
    prompt: &'static str,
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
    pub fn new(
        writer: W,
        command_buffer: CommandBuffer,
        history_buffer: HistoryBuffer,
    ) -> Result<Self, E> {
        let history: History<HistoryBuffer> = History::new(history_buffer);

        let mut cli = Self {
            editor: Some(Editor::new(command_buffer)),
            history,
            input_generator: Some(InputGenerator::new()),
            prompt: PROMPT,
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

    fn on_text_input(&mut self, editor: &mut Editor<CommandBuffer>, text: &str) -> Result<(), E> {
        if let Some(c) = editor.insert(text) {
            //TODO: cursor position not at end
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

                let text = editor.text();
                self.history.push(text);
                let text = editor.text_mut();

                if let Some(tokens) = Tokens::new(text) {
                    self.process_input::<C, _>(tokens, processor)?;
                }

                editor.clear();

                self.writer.flush_str(self.prompt)?;
            }
            ControlInput::Tab => {
                self.process_autocomplete::<C>(editor)?;
            }
            ControlInput::Backspace => {
                if editor.move_left() {
                    editor.remove();
                    self.writer.flush_str("\x08 \x08")?;
                }
            }
            ControlInput::Down => self.navigate_history(editor, NavigateHistory::Newer)?,
            ControlInput::Up => self.navigate_history(editor, NavigateHistory::Older)?,
        }

        Ok(())
    }

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
            let input_len = editor.len();
            editor.clear();
            editor.insert(element);
            self.clear_line(input_len, false)?;

            self.writer.flush_str(editor.text())?;
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
                    // SAFETY: "help" starts with name, so name cannot be longer
                    let autocompleted = unsafe { "help".get_unchecked(name.len()..) };
                    autocompletion.merge_autocompletion(autocompleted)
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
