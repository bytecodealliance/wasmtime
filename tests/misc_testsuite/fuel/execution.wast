;;! reference_types = true
;;! exceptions = true
;;! extended_const = true
;;! consume_fuel = true

(module $Fuel
  (import "wasmtime" "set-fuel" (func $set (param i64)))
  (import "wasmtime" "get-fuel" (func $get (result i64)))
  (export "set-fuel" (func $set))
  (export "get-fuel" (func $get)))
(register "fuel" $Fuel)

;; The store's fuel can't go negative: it reads as 0 after overshooting, and
;; the next call runs out of fuel.
(module
  (func (export "add2") (param $n i32) (result i32)
    (i32.add (local.get $n) (i32.const 2))))
(invoke $Fuel "set-fuel" (i64.const 6))
(assert_return (invoke "add2" (i32.const 10)) (i32.const 12))
(assert_return (invoke $Fuel "get-fuel") (i64.const 2))
(assert_return (invoke "add2" (i32.const 10)) (i32.const 12))
(assert_return (invoke $Fuel "get-fuel") (i64.const 0))
(assert_trap (invoke "add2" (i32.const 10)) "all fuel consumed")

;; Function entry alone uses up a single unit of fuel.
(module
  (func (export "main")))
(invoke $Fuel "set-fuel" (i64.const 1))
(assert_trap (invoke "main") "all fuel consumed")

;; A host call sees the fuel wasm has used so far, and wasm picks up fuel that
;; the host changes.
(module
  (import "fuel" "get-fuel" (func $get (result i64)))
  (import "fuel" "set-fuel" (func $set (param i64)))
  (func (export "run")
    (if (i64.ne (call $get) (i64.const 9998)) (then unreachable))
    (call $set (i64.const 1))
    (call $other))
  (func $other))
(invoke $Fuel "set-fuel" (i64.const 10000))
(assert_trap (invoke "run") "all fuel consumed")

;; Fuel used before an access that always traps is saved before the trap.
(module
  (memory 1 1)
  (func (export "f") (param i32) (result i32)
    local.get 0
    local.get 0
    i32.add
    ;; This offset is larger than the memory's maximum size, so the access
    ;; unconditionally traps.
    i32.load8_s offset=0xffffffff))
(invoke $Fuel "set-fuel" (i64.const 1000))
(assert_trap (invoke "f" (i32.const 0)) "out of bounds memory access")
(assert_return (invoke $Fuel "get-fuel") (i64.const 995))

;; Unreachable code after `return`, including a `try_table`, uses no fuel:
;; only function entry and `i32.const` do.
(module
  (func (export "run") (result i32)
    i32.const 42
    return
    (try_table)))
(invoke $Fuel "set-fuel" (i64.const 100))
(assert_return (invoke "run") (i32.const 42))
(assert_return (invoke $Fuel "get-fuel") (i64.const 98))

;; Function entry, `try_table`, `local.get`, `if` and `i32.const` each use one
;; unit of fuel, as do `throw` and `throw_ref` when they run.
(module
  (tag $e (param i32))

  (func (export "throw") (param i32) (result i32)
    (block $catch (result i32)
      (try_table (result i32) (catch $e $catch)
        local.get 0
        if
          i32.const 42
          throw $e
        end
        i32.const 7)))

  (func (export "throw_ref") (result i32)
    (block $catch (result i32)
      (try_table (result i32) (catch $e $catch)
        (block $rethrow (result i32 exnref)
          (try_table (result i32 exnref) (catch_ref $e $rethrow)
            i32.const 42
            throw $e))
        throw_ref))))
(invoke $Fuel "set-fuel" (i64.const 100))
(assert_return (invoke "throw" (i32.const 0)) (i32.const 7))
(assert_return (invoke $Fuel "get-fuel") (i64.const 95))
(invoke $Fuel "set-fuel" (i64.const 100))
(assert_return (invoke "throw" (i32.const 1)) (i32.const 42))
(assert_return (invoke $Fuel "get-fuel") (i64.const 94))
(invoke $Fuel "set-fuel" (i64.const 100))
(assert_return (invoke "throw_ref") (i32.const 42))
(assert_return (invoke $Fuel "get-fuel") (i64.const 94))

;; Fuel is saved and reloaded around calls inside `try_table`, both when they
;; return normally and when they throw, so deep recursion through them still
;; runs out of fuel (#14407).
(module
  (tag $e)
  (func $run (export "run") (param $cnt i32) (param $depth i32)
    (local $i i32)
    (local.set $i (local.get $cnt))
    local.get $depth
    if
      (local.set $depth (i32.sub (local.get $depth) (i32.const 1)))
      loop $l
        (block $catch
          (try_table (catch $e $catch)
            (call $run (local.get $cnt) (local.get $depth))))
        (local.tee $i (i32.sub (local.get $i) (i32.const 1)))
        if br $l end
      end
    end))
(invoke $Fuel "set-fuel" (i64.const 100000))
(assert_trap (invoke "run" (i32.const 10) (i32.const 10)) "all fuel consumed")

(module
  (tag $e)
  (func $run (param $cnt i32) (param $depth i32)
    (local $i i32)
    (local.set $i (local.get $cnt))
    local.get $depth
    if
      (local.set $depth (i32.sub (local.get $depth) (i32.const 1)))
      loop $l
        (block $catch
          (try_table (catch $e $catch)
            (call $run (local.get $cnt) (local.get $depth))))
        (local.tee $i (i32.sub (local.get $i) (i32.const 1)))
        if br $l end
      end
    end
    throw $e)
  (func (export "run") (param $cnt i32) (param $depth i32)
    (block $catch
      (try_table (catch $e $catch)
        (call $run (local.get $cnt) (local.get $depth))))))
(invoke $Fuel "set-fuel" (i64.const 100000))
(assert_trap (invoke "run" (i32.const 10) (i32.const 10)) "all fuel consumed")

;; Without a start function, the module's startup function is the only place
;; a const expr runs, and it must still flush the const expr's fuel: one unit
;; for entry, plus `i32.const`, `i32.const` and `i32.add`.
(invoke $Fuel "set-fuel" (i64.const 10000))
(module
  (global $g (export "g") i32 (i32.add (i32.const 1) (i32.const 2))))
(assert_return (invoke $Fuel "get-fuel") (i64.const 9996))
(assert_return (get "g") (i32.const 3))
