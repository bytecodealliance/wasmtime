;;! bulk_memory = true
;;! reference_types = true
;;! exceptions = true
;;! consume_fuel = true

;; Each test sets the store's fuel to 1000, instantiates a module (usually
;; running its start function), and checks exactly how much fuel is left.
(module $Fuel
  (import "wasmtime" "set-fuel" (func $set (param i64)))
  (import "wasmtime" "get-fuel" (func $get (result i64)))
  (export "set-fuel" (func $set))
  (export "get-fuel" (func $get)))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module)
(assert_return (invoke $Fuel "get-fuel") (i64.const 1000))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (func $f)
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 997))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (func $f
    i32.const 0
    drop
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 996))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (func $f
    block
    end
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 997))

(invoke $Fuel "set-fuel" (i64.const 1000))
(assert_trap
  (module
    (func $f
      unreachable
    )
    (start $f))
  "wasm `unreachable` instruction executed")
(assert_return (invoke $Fuel "get-fuel") (i64.const 997))

(invoke $Fuel "set-fuel" (i64.const 1000))
(assert_trap
  (module
    (func $f
      i32.const 0
      i32.const 0
      i32.const 0
      i32.const 0
      i32.const 0
      i32.const 0
      unreachable
    )
    (start $f))
  "wasm `unreachable` instruction executed")
(assert_return (invoke $Fuel "get-fuel") (i64.const 991))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (func $f
    return
    i32.const 0
    i32.const 0
    i32.const 0
    i32.const 0
    i32.const 0
    i32.const 0
    unreachable
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 997))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (func $f
    i32.const 0
    if
      call $f
    end
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 995))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (func $f
    i32.const 1
    if
      i32.const 0
      drop
    end
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 994))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (func $f
    i32.const 1
    if
      i32.const 0
      drop
    else
      call $f
    end
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 994))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (func $f
    i32.const 0
    if
      call $f
    else
      i32.const 0
      drop
    end
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 994))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (func $f
    block
      i32.const 1
      br_if 0
      i32.const 0
      drop
    end
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 995))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (func $f
    block
      i32.const 0
      br_if 0
      i32.const 0
      drop
    end
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 994))

;; count code before unreachable
(invoke $Fuel "set-fuel" (i64.const 1000))
(assert_trap
  (module
    (func $f
      i32.const 0
      unreachable
    )
    (start $f))
  "wasm `unreachable` instruction executed")
(assert_return (invoke $Fuel "get-fuel") (i64.const 996))

;; count code before return
(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (func $f
    i32.const 0
    return
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 996))

;; cross-function fuel works
(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (func $f
    call $other
  )
  (func $other)
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 995))
(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (func $f
    i32.const 0
    call $other
    i32.const 0
    drop
  )
  (func $other (param i32))
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 993))
(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (func $f
    call $other
    drop
  )
  (func $other (result i32)
    i32.const 0
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 994))
(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (func $f
    i32.const 0
    call_indirect
  )
  (func $other)
  (table funcref (elem $other))
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 994))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (tag $e)
  (func $f
    block $catch
      try_table (catch $e $catch)
        call $other
      end
    end
  )
  (func $other)
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 994))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (tag $e)
  (func $f
    block $catch
      try_table (catch $e $catch)
        call $other
      end
    end
  )
  (func $other
    i32.const 0
    i32.const 0
    i32.const 0
    drop
    drop
    drop
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 991))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (tag $e)
  (func $f
    block $catch
      try_table (catch $e $catch)
        call $other
      end
    end
  )
  (func $other
    throw $e
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 993))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (tag $e)
  (func $f
    block $catch
      try_table (catch $e $catch)
        call $other
      end
    end
  )
  (func $other
    i32.const 0
    i32.const 0
    i32.const 0
    drop
    drop
    drop
    throw $e
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 990))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (tag $e)
  (func $f
    block $catch
      try_table (catch_all $catch)
        call $other
      end
    end
  )
  (func $other
    i32.const 0
    i32.const 0
    i32.const 0
    drop
    drop
    drop
    throw $e
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 990))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (tag $e)
  (func $f
    block $catch (result exnref)
      try_table (catch_ref $e $catch)
        call $other
      end
      unreachable
    end
    drop
  )
  (func $other
    i32.const 0
    i32.const 0
    i32.const 0
    drop
    drop
    drop
    throw $e
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 990))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (tag $e (param i32))
  (func $f
    block $catch (result i32)
      try_table (result i32) (catch $e $catch)
        call $other
      end
    end
    i32.const 1
    i32.add
    drop
  )
  (func $other (result i32)
    i32.const 0
    i32.const 0
    i32.const 0
    drop
    drop
    throw $e
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 988))

;; loops!
(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (func $f
    loop
    end
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 997))
;; 5 loop instructions, 10 iterations, 2 header instrs, 1 func
(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (func $f
    (local i32)
    i32.const 10
    local.set 0

    loop
      local.get 0
      i32.const 1
      i32.sub
      local.tee 0
      br_if 0
    end
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 945))
