# filetests

Filetests is a crate that contains multiple test suites for testing
various parts of cranelift. Each folder under `cranelift/filetests/filetests` is a different
test suite that tests different parts.

## Adding a runtest

One of the available testsuites is the "runtest" testsuite. Its goal is to compile some piece
of clif code, run it and ensure that what comes out is what we expect. 

To build a run test you can add the following to a file:

```
test interpret
test run
target x86_64
target aarch64
target s390x

function %band_f32(f32, f32) -> f32 {
block0(v0: f32, v1: f32):
    v2 = band v0, v1
    return v2
}
; run: %band_f32(0x0.5, 0x1.0) == 0x1.5
```

Since this is a run test for `band` we can put it in: `runtests/band.clif`.
Once we have the file in the test suite we can run it by invoking: `cargo run -- test filetests/filetests/runtests/band.clif` from the cranelift directory. 


The first lines tell `clif-util` what kind of tests we want to run on this file. 
`test interpret` invokes the interpreter and checks if the conditions in the `; run` comments pass. `test run` does the same, but compiles the file and runs it as a native binary. 

For more information about testing see [testing.md](../docs/testing.md).
