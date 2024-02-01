It looks like you are changing Wasmtime's configuration options. Make sure to
complete this check list:

* [ ] If you added a new `Config` method, you wrote extensive documentation for
      it.

  <details>

  Our documentation should be of the following form:

  ```text
  Short, simple summary sentence.

  More details. These details can be multiple paragraphs. There should be
  information about not just the method, but its parameters and results as
  well.

  Is this method fallible? If so, when can it return an error?

  Can this method panic? If so, when does it panic?

  # Example

  Optional example here.
  ```

  </details>

* [ ] If you added a new `Config` method, or modified an existing one, you
  ensured that this configuration is exercised by the fuzz targets.

  <details>

  For example, if you expose a new strategy for allocating the next instance
  slot inside the pooling allocator, you should ensure that at least one of our
  fuzz targets exercises that new strategy.

  Often, all that is required of you is to ensure that there is a knob for this
  configuration option in [`wasmtime_fuzzing::Config`][fuzzing-config] (or one
  of its nested `struct`s).

  Rarely, this may require authoring a new fuzz target to specifically test this
  configuration. See [our docs on fuzzing][fuzzing-docs] for more details.

  </details>

* [ ] If you are enabling a configuration option by default, make sure that it
  has been fuzzed for at least two weeks before turning it on by default.

[fuzzing-config]: https://github.com/bytecodealliance/wasmtime/blob/ca0e8d0a1d8cefc0496dba2f77a670571d8fdcab/crates/fuzzing/src/generators.rs#L182-L194
[fuzzing-docs]: https://docs.wasmtime.dev/contributing-fuzzing.html
