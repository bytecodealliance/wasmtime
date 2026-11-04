;;! multi_memory = true

;; Strings passed between components are copied by the host, so they're
;; limited by the store's hostcall fuel. Each test below runs with exactly the
;; fuel it needs and then traps with one byte less.

;; `$b` passes a 64 KiB string to `$a`.
(component definition $one-string
  (import "wasmtime" (instance $wasmtime
    (export "set-hostcall-fuel" (func (param "fuel" u64)))
  ))
  (component $a
    (core module $m
      (memory (export "mem") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 0)
      (func (export "f") (param i32 i32)))
    (core instance $i (instantiate $m))
    (func (export "f") (param "s" string)
      (canon lift (core func $i "f") (memory (core memory $i "mem"))
        (realloc (core func $i "realloc")))))
  (component $b
    (import "f" (func $f (param "s" string)))
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) (memory (core memory $libc "mem"))))
    (core module $m
      (import "" "f" (func $f (param i32 i32)))
      (func (export "run") (call $f (i32.const 0) (i32.const 65536))))
    (core instance $i (instantiate $m (with "" (instance (export "f" (func $f))))))
    (func (export "run") (canon lift (core func $i "run"))))
  (instance $a (instantiate $a))
  (instance $b (instantiate $b (with "f" (func $a "f"))))
  (core func $set-hostcall-fuel (canon lower (func $wasmtime "set-hostcall-fuel")))
  (core module $fuel
    (import "" "set" (func $set (param i64)))
    (func (export "set") (param i64) (call $set (local.get 0))))
  (core instance $fuel (instantiate $fuel
    (with "" (instance (export "set" (func $set-hostcall-fuel))))))
  (func (export "set-hostcall-fuel") (param "fuel" u64)
    (canon lift (core func $fuel "set")))
  (export "run" (func $b "run")))

(component instance $i $one-string)
(assert_return (invoke "set-hostcall-fuel" (u64.const 65536)))
(assert_return (invoke "run"))
(component instance $i $one-string)
(assert_return (invoke "set-hostcall-fuel" (u64.const 65535)))
(assert_trap (invoke "run") "fuel allocated for hostcalls has been exhausted")

;; `$b` passes two 64 KiB strings to `$a` in one call, which share one budget.
(component definition $two-strings
  (import "wasmtime" (instance $wasmtime
    (export "set-hostcall-fuel" (func (param "fuel" u64)))
  ))
  (component $a
    (core module $m
      (memory (export "mem") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 0)
      (func (export "f") (param i32 i32 i32 i32)))
    (core instance $i (instantiate $m))
    (func (export "f") (param "s1" string) (param "s2" string)
      (canon lift (core func $i "f") (memory (core memory $i "mem"))
        (realloc (core func $i "realloc")))))
  (component $b
    (import "f" (func $f (param "s1" string) (param "s2" string)))
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) (memory (core memory $libc "mem"))))
    (core module $m
      (import "" "f" (func $f (param i32 i32 i32 i32)))
      (func (export "run")
        (call $f (i32.const 0) (i32.const 65536) (i32.const 0) (i32.const 65536))))
    (core instance $i (instantiate $m (with "" (instance (export "f" (func $f))))))
    (func (export "run") (canon lift (core func $i "run"))))
  (instance $a (instantiate $a))
  (instance $b (instantiate $b (with "f" (func $a "f"))))
  (core func $set-hostcall-fuel (canon lower (func $wasmtime "set-hostcall-fuel")))
  (core module $fuel
    (import "" "set" (func $set (param i64)))
    (func (export "set") (param i64) (call $set (local.get 0))))
  (core instance $fuel (instantiate $fuel
    (with "" (instance (export "set" (func $set-hostcall-fuel))))))
  (func (export "set-hostcall-fuel") (param "fuel" u64)
    (canon lift (core func $fuel "set")))
  (export "run" (func $b "run")))

(component instance $i $two-strings)
(assert_return (invoke "set-hostcall-fuel" (u64.const 131072)))
(assert_return (invoke "run"))
(component instance $i $two-strings)
(assert_return (invoke "set-hostcall-fuel" (u64.const 131071)))
(assert_trap (invoke "run") "fuel allocated for hostcalls has been exhausted")

;; `$b` passes a 64 KiB string to `$a`, which returns a 64 KiB string. Params and
;; results each get their own budget.
(component definition $roundtrip
  (import "wasmtime" (instance $wasmtime
    (export "set-hostcall-fuel" (func (param "fuel" u64)))
  ))
  (component $a
    (core module $m
      (memory (export "mem") 2)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 0)
      (func (export "f") (param i32 i32) (result i32)
        (i32.store (i32.const 65536) (i32.const 0))
        (i32.store (i32.const 65540) (i32.const 65536))
        i32.const 65536))
    (core instance $i (instantiate $m))
    (func (export "f") (param "s" string) (result string)
      (canon lift (core func $i "f") (memory (core memory $i "mem"))
        (realloc (core func $i "realloc")))))
  (component $b
    (import "f" (func $f (param "s" string) (result string)))
    (core module $libc
      (memory (export "mem") 2)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 0))
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) (memory (core memory $libc "mem"))
      (realloc (core func $libc "realloc"))))
    (core module $m
      (import "" "f" (func $f (param i32 i32 i32)))
      (func (export "run") (call $f (i32.const 0) (i32.const 65536) (i32.const 65536))))
    (core instance $i (instantiate $m (with "" (instance (export "f" (func $f))))))
    (func (export "run") (canon lift (core func $i "run"))))
  (instance $a (instantiate $a))
  (instance $b (instantiate $b (with "f" (func $a "f"))))
  (core func $set-hostcall-fuel (canon lower (func $wasmtime "set-hostcall-fuel")))
  (core module $fuel
    (import "" "set" (func $set (param i64)))
    (func (export "set") (param i64) (call $set (local.get 0))))
  (core instance $fuel (instantiate $fuel
    (with "" (instance (export "set" (func $set-hostcall-fuel))))))
  (func (export "set-hostcall-fuel") (param "fuel" u64)
    (canon lift (core func $fuel "set")))
  (export "run" (func $b "run")))

(component instance $i $roundtrip)
(assert_return (invoke "set-hostcall-fuel" (u64.const 65536)))
(assert_return (invoke "run"))
(component instance $i $roundtrip)
(assert_return (invoke "set-hostcall-fuel" (u64.const 65535)))
(assert_trap (invoke "run") "fuel allocated for hostcalls has been exhausted")
