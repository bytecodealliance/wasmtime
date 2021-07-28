This directory contains the necessary parts for building a library with FFI
access to the Wasm spec interpreter. Its major parts:
 - `spec`: the Wasm spec code as a Git submodule (you may need to retrieve it:
   `git clone https://github.com/bytecodealliance/wasm-spec-mirror).
 - `interpret.ml`: a shim layer for calling the Wasm spec code and exposing it
   for FFI access
 - `Makefile`: the steps for gluing these pieces together into a static library
