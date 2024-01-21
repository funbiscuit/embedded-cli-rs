# Arduino example

This example shows how to build cli with Arduino Nano.
Another Arduino can also be used, but you will have to tweak configs.
Example uses ~14KiB of ROM and ~0.5KiB of static RAM.
Most of RAM is taken by derived implementations for help and autocomplete
that don't use progmem. In future this should be fixed.

# Running
## Linux

Run with:
```shell
RAVEDUDE_PORT=/dev/ttyUSB0 cargo run --release
```

After flashing is completed, disconnect and reconnect with more
appropriate terminal. For example [tio](https://github.com/tio/tio):

```shell
tio /dev/ttyUSB0 --map ODELBS
```

# Memory usage

Memory usage might vary depending on compiler version, build environment and library version.
To find out total ROM usage run:

```shell
cargo bloat --release
```

Example output:
```
File  .text    Size        Crate Name
1.6%  56.9%  8.1KiB  arduino_cli arduino_cli::try_run
0.1%   3.7%    538B embedded_cli embedded_cli::token::Tokens::new
0.1%   3.6%    530B embedded_cli embedded_cli::help::HelpRequest::from_tokens
0.1%   3.2%    472B embedded_cli embedded_cli::token::Tokens::remove
0.1%   3.2%    468B embedded_cli embedded_cli::cli::Cli<W,E,CommandBuffer,HistoryBuffer>::navigate_history
0.7%  26.0%  3.7KiB              And 34 smaller methods. Use -n N to show more.
2.9% 100.0% 14.2KiB              .text section size, the file size is 493.3KiB
```

To find total static memory usage:

```shell
cargo build --release && \
  avr-nm -Crtd --size-sort \
    target/avr-atmega328p/release/arduino-cli.elf \
  | grep -i ' [dbv] ' \
  |  awk -F " " '{Total=Total+$1} END{print "RAM usage: " Total}' -
```

Example output:
```
RAM usage: 506
```

To further analyze used space:
```
avr-objdump -s -j .data target/avr-atmega328p/release/arduino-cli.elf
```
