#!/bin/bash

cargo build
for name in add counter add_func fibonacci locals locals_simple fibonacci_recursive
do
	echo $name;
	../target/debug/clif-util wasm --target sparc-unknown-unknown ../../zkwasm/data/$name.wat > data/$name.zkasm
done
