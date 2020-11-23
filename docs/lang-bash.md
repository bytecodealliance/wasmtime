# Using WebAssembly from Bash

## Getting started and simple example

First up you'll want to start a new module:

```text
$ mkdir -p gcd-bash
$ cd gcd-bash
$ touch gcd.wat gcd.sh
```

Next, copy this example WebAssembly text module into your project. It exports a function for calculating the greatest common denominator of two numbers.

## `gcd.wat`

```wat
{{#include ../examples/gcd.wat}}
```

## `gcd.wast` (for Unit Testing)

```wat
(assert_return (invoke "gcd" (i32.const 27) (i32.const 3)) (i32.const 3))
(assert_return (invoke "gcd" (i32.const 6) (i32.const 27)) (i32.const 3))
```

Create a bash script that will invoke GCD three times, and runs unit tests.

## `gcd.sh`

```bash
#!/bin/bash

function gcd_unit_tests() {
  local result=$(wasmtime wast examples/gcd.wat gcd.wast 2>&1)
  if [ ${#result} -eq 0 ]; then
    echo "Unit tests passed"
  else
    echo "Unit tests failed: $result"
  fi
}

function gcd() {
  # Cast to number; default = 0
  local x=$(($1))
  local y=$(($2))
  # Invoke GCD from module; suppress stderr
  local result=$(wasmtime examples/gcd.wat --invoke gcd $x $y 2>/dev/null)
  echo "$result"
}

# main
gcd_unit_tests

for num in "27 6" "6 27" "42 12"; do
  set -- $num
  echo "gcd($1, $2) = $(gcd "$1" "$2")"
done
```
