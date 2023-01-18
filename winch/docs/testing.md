# Testing Winch

Winch is tested through integration testing using the `winch-filetests` crate
and manual exploratory testing. A CLI is available to run these tests
conveniently. To add the `winch-tools` binary to your `PATH`, run `cargo install
--path winch` from the root of `wasmtime`. The CLI provides two commands: `test`
and `compile`. To see the help text for each command, run `winch-tools test
--help` or `winch-tools compile --help`.

## Integration Testing (`winch-tools test`)

The `test` command will run a suite of tests that validates Winch output for a
WebAssembly module is consistent with our expectations.

### Running `test`

Running `winch-tools test` will run all integration tests in the
`winch-filetests` crate. All arguments following two dashes (`--`) will be
passed directly to `cargo test -p winch-filetests`. This will allow you to
configure the tests to run based on your requirements. All tests in the
`winch-filetests` crate get named in the following convention:
`winch_filetests_${filepath}`. This makes it possible to filter if you don't
want to run the entire suite.

If the output of Winch changes for a test in a run due to code updates, the test
will fail and the difference between the two outputs will be shown. If the new
output is expected, the tests can be re-run with an `WINCH_TEST_BLESS`
environment variable set to `1`.

### Adding a test

To add new tests, create a `.wat` file in the `winch/filetests/filetests` folder
in the following format:

```wat
;;! target = "x86_64"
(module
  (func (result i32)
    (i32.const 42)
  )
)
```

It is encouraged to use folders to organize tests. For example, tests targeting
the x86_64 architecture can be placed in the `winch/filetests/filetests/x64`.

The first block of comments are a TOML compatible configuration passed to Winch
during compilation with a `!` at the start of each line. The body of the file
will be the subject of the test. A final block of comments is reserved for the
output of the compilation, and it will be used to compare the output of the
current run with the output of previous runs.

## Manual Exploratory Tests (`winch-tools compile`)

The `compile` command will run Winch for particular architecture against
provided input file, and print the disassembled output to the console. Only
`.wat` files are supported.

### Running `compile`

```bash
winch-tools compile $wat_file --target $target_triple
```
