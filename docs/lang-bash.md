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

Create a bash script that will invoke GCD three times.

## `gcd.sh`

```bash
#!/bin/bash

function gcd() {
  # Cast to number; default = 0
  local x=$(($1))
  local y=$(($2))
  # Invoke GCD from module; suppress stderr
  local result=$(wasmtime examples/gcd.wat --invoke gcd $x $y 2>/dev/null)
  echo "$result"
}

# main
for num in "27 6" "6 27" "42 12"; do
  set -- $num
  echo "gcd($1, $2) = $(gcd "$1" "$2")"
done
```
