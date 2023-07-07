# byte-array-literals

This crate exists to solve a very peculiar problem for the
`wasi-preview1-component-adapter`: we want to use string literals in our
source code, but the resulting binary (when compiled for
wasm32-unknown-unknown) cannot contain any data sections.

The answer that @sunfishcode discovered is that these string literals, if
represented as an array of u8 literals, these will somehow not end up in the
data section, at least when compiled with opt-level='s' on today's rustc
(1.69.0). So, this crate exists to transform these literals using a proc
macro.

It is very possible this cheat code will abruptly stop working in some future
compiler, but we'll cross that bridge when we get to it.
