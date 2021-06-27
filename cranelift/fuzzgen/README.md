# `cranelift-fuzzgen`

This crate implements a generator to create random cranelift modules

## `gen-testcase`

This is a util that allows you to quickly test changes in the fuzzgen library.

It can be run without input, in which case it will generate a random cranelift module, a few test cases,
and will print the result as a clif file to stdout.

You can run it in this mode by running the following command in the current directory:
```
cargo run --bin gen_testcase 
```


If you pass in a fuzzer artifact file, it will generate the clif module and test inputs
for that fuzzer run.

You can run it in this mode by running the following command in the current directory:
```
cargo run --bin gen_testcase ../fuzz/artifacts/fuzzgen/crash-1ef624482d620e43e6d34da1afe46267fc695d9b
```
