// Compile with:
//   clang --target=wasm32 fib-wasm.c -o fib-wasm.wasm -gdwarf-4 \
//     -Wl,--no-entry,--export=fib -nostdlib -fdebug-prefix-map=$PWD=.
//
//   clang --target=wasm32 fib-wasm.c -o fib-wasm-dwarf5.wasm -gdwarf-5 \
//     -Wl,--no-entry,--export=fib -nostdlib -fdebug-prefix-map=$PWD=.

int fib(int n) {
  int t, a = 0, b = 1;
  for (int i = 0; i < n; i++) {
    t = a;
    a = b;
    b += t;
  }
  return b;
}

// Placed at the end so that line numbers are stable.
//
//   clang --target=wasm32 fib-wasm.c -o fib-wasm-split4.wasm \
//     -gdwarf-4 -gsplit-dwarf \
//     -Wl,--no-entry,--export=fib -nostdlib -fdebug-prefix-map=$PWD=.
//   llvm-dwp -e fib-wasm-split4.wasm -o fib-wasm-split4.dwp
