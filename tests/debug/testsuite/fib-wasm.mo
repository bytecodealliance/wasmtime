// Compile like:
// moc fib-wasm.mo -o fib-wasm.mo.wasm -wasi-system-api

func fib(n : Nat32) : Nat32
 = switch n {
     case 0 0;
     case 1 1;
     case _ fib(n - 2) + fib (n - 1)
   };

assert (fib(10) == (55 : Nat32));
