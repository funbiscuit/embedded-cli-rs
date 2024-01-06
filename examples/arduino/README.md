# Arduino example

This example shows how to build cli with Arduino Nano.
Another Arduino can also be used, but you will have to tweak configs.
Example uses ~17KiB of ROM and 1.1KiB of static RAM.
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

Memory usage might vary depending on compiler version and build environment.
To find out total ROM usage run:

```shell
cargo bloat --release
```

Example output:
```
File  .text    Size        Crate Name
1.7%  56.5%  9.1KiB  arduino_cli arduino_cli::__avr_device_rt_main
0.2%   7.8%  1.3KiB         core <core::iter::adapters::skip::Skip<I> as core::iter::traits::iterator::Iterator>::next
0.2%   6.3%  1.0KiB embedded_cli embedded_cli::token::Tokens::remove
0.2%   6.2%   1016B embedded_cli embedded_cli::autocomplete::Autocompletion::merge_autocompletion
0.1%   2.9%    474B         core core::str::count::do_count_chars
0.5%  17.5%  2.8KiB              And 26 smaller methods. Use -n N to show more.
3.1% 100.0% 16.1KiB              .text section size, the file size is 523.3KiB
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
RAM usage: 1062
```

To further analyze used space:
```
avr-objdump -s -j .data target/avr-atmega328p/release/arduino-cli.elf
```
