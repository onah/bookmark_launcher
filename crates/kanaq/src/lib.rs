//! Pure logic behind the `kanaq` subcommands, kept free of I/O so it can be
//! unit tested directly. The CLI wrappers live in `commands/`.

pub mod complete;
pub mod filter;
