# Arduino example

This example shows how to build cli with Arduino Nano.
Another Arduino can also be used, but you will have to tweak configs.
Example uses ~16KiB of ROM and ~0.6KiB of static RAM.
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
You can run `memory.sh` script to calculate memory usage of this arduino example with different activated features
of cli library.

To analyze ROM usage:

```shell
cargo bloat --release
```

To analyze static RAM:
```shell
avr-objdump -s -j .data target/avr-atmega328p/release/arduino-cli.elf
```
