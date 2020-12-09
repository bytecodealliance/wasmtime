Source code used to create `/wizer/benches/uap_bench.{control,wizer}.wasm`.

Creates a `RegexSet` from the user agent parsing regexes from the browserscope
project in the initialization function and then tests whether the input string
is a known user agent.

Rebuild via:

```
$ ./build.sh
```
