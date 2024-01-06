# embedded-cli

> **Command Line Interface for embedded systems**

Dual-licensed under [Apache 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT).

## Features

* Static allocation
* Configurable memory usage
* Declaration of commands with enums
* Parsing of arguments to common types
* Autocompletion of command names (with tab)
* History (navigate with up and down keypress)
* Help (generated from doc comments)
* Formatted write with [ufmt](https://github.com/japaric/ufmt)
* No panicking branches in generated code, when optimized
* Any byte-stream interface is supported (`embedded_io::Write` as output stream, input bytes are given one-by-one)
