use core::{convert::Infallible, fmt::Debug};

use embedded_io::{Error, Write};

use crate::{buffer::Buffer, cli::Cli, writer::EmptyWriter};

pub const DEFAULT_CMD_LEN: usize = 40;
pub const DEFAULT_HISTORY_LEN: usize = 100;
pub const DEFAULT_PROMPT: &str = "$ ";

pub struct CliBuilder<W: Write<Error = E>, E: Error, CommandBuffer: Buffer, HistoryBuffer: Buffer> {
    pub(crate) command_buffer: CommandBuffer,
    pub(crate) history_buffer: HistoryBuffer,
    pub(crate) prompt: &'static str,
    pub(crate) writer: W,
}

impl<W, E, CommandBuffer, HistoryBuffer> Debug for CliBuilder<W, E, CommandBuffer, HistoryBuffer>
where
    W: Write<Error = E>,
    E: Error,
    CommandBuffer: Buffer,
    HistoryBuffer: Buffer,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CliBuilder")
            .field("command_buffer", &self.command_buffer.as_slice())
            .field("history_buffer", &self.history_buffer.as_slice())
            .finish()
    }
}

impl<W, E, CommandBuffer, HistoryBuffer> CliBuilder<W, E, CommandBuffer, HistoryBuffer>
where
    W: Write<Error = E>,
    E: Error,
    CommandBuffer: Buffer,
    HistoryBuffer: Buffer,
{
    pub fn build(self) -> Result<Cli<W, E, CommandBuffer, HistoryBuffer>, E> {
        Cli::from_builder(self)
    }

    pub fn command_buffer<B: Buffer>(
        self,
        command_buffer: B,
    ) -> CliBuilder<W, E, B, HistoryBuffer> {
        CliBuilder {
            command_buffer,
            history_buffer: self.history_buffer,
            writer: self.writer,
            prompt: self.prompt,
        }
    }

    pub fn history_buffer<B: Buffer>(
        self,
        history_buffer: B,
    ) -> CliBuilder<W, E, CommandBuffer, B> {
        CliBuilder {
            command_buffer: self.command_buffer,
            history_buffer,
            writer: self.writer,
            prompt: self.prompt,
        }
    }

    pub fn prompt(self, prompt: &'static str) -> Self {
        CliBuilder {
            command_buffer: self.command_buffer,
            history_buffer: self.history_buffer,
            writer: self.writer,
            prompt,
        }
    }

    pub fn writer<T: Write<Error = TE>, TE: Error>(
        self,
        writer: T,
    ) -> CliBuilder<T, TE, CommandBuffer, HistoryBuffer> {
        CliBuilder {
            command_buffer: self.command_buffer,
            history_buffer: self.history_buffer,
            writer,
            prompt: self.prompt,
        }
    }
}

impl Default
    for CliBuilder<EmptyWriter, Infallible, [u8; DEFAULT_CMD_LEN], [u8; DEFAULT_HISTORY_LEN]>
{
    fn default() -> Self {
        Self {
            command_buffer: [0; DEFAULT_CMD_LEN],
            history_buffer: [0; DEFAULT_HISTORY_LEN],
            writer: EmptyWriter,
            prompt: DEFAULT_PROMPT,
        }
    }
}
