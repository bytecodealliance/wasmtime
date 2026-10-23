;;! multi_memory = true

;; A component instance calling a function that it itself lifted traps, as the
;; source and destination of transferred values may overlap in the same linear
;; memory.

;; string parameter, where `realloc` returns the source string's own address
(component
  (core module $M
    (memory (export "mem") 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      i32.const 100)
    (func (export "f") (param i32 i32) (result i32) local.get 1))
  (core instance $m (instantiate $M))
  (func $f (param "s" string) (result u32)
    (canon lift (core func $m "f") (memory (core memory $m "mem"))
      (realloc (core func $m "realloc")) string-encoding=utf8))
  (core func $f' (canon lower (func $f) (memory (core memory $m "mem"))
      (realloc (core func $m "realloc")) string-encoding=utf8))
  (core module $N
    (import "" "f" (func $f (param i32 i32) (result i32)))
    (import "" "mem" (memory 1))
    (data (i32.const 100) "hello world")
    (func (export "run") (result i32)
      (call $f (i32.const 100) (i32.const 11))))
  (core instance $n (instantiate $N (with "" (instance
    (export "f" (func $f'))
    (export "mem" (memory $m "mem"))))))
  (func (export "run") (result u32) (canon lift (core func $n "run")))
)
(assert_trap (invoke "run")
  "cannot enter component instance")

;; string nested in a result, with differing string encodings, and with the
;; caller referring to the memory through a re-exporting core instance
(component
  (core module $M
    (memory (export "mem") 1)
    (data (i32.const 200) "\01\00\00\00\2c\01\00\00\02\00\00\00")
    (data (i32.const 300) "h\00i\00")
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      i32.const 100)
    (func (export "f") (result i32) i32.const 200))
  (core instance $m (instantiate $M))
  (core module $Reexport
    (import "" "mem" (memory 1))
    (export "mem" (memory 0)))
  (core instance $r (instantiate $Reexport (with "" (instance
    (export "mem" (memory $m "mem"))))))
  (func $f (result (option string))
    (canon lift (core func $m "f") (memory (core memory $m "mem"))
      string-encoding=utf16))
  (core func $f' (canon lower (func $f) (memory (core memory $r "mem"))
      (realloc (core func $m "realloc")) string-encoding=utf8))
  (core module $N
    (import "" "f" (func $f (param i32)))
    (func (export "run") (call $f (i32.const 0))))
  (core instance $n (instantiate $N (with "" (instance
    (export "f" (func $f'))))))
  (func (export "run") (canon lift (core func $n "run")))
)
(assert_trap (invoke "run")
  "cannot enter component instance")

;; lists also trap
(component
  (core module $M
    (memory (export "mem") 1)
    (data (i32.const 100) "\01\02\03")
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      i32.const 100)
    (func (export "f") (param i32 i32) (result i32)
      (i32.add
        (i32.load8_u (local.get 0))
        (i32.load8_u offset=2 (local.get 0)))))
  (core instance $m (instantiate $M))
  (func $f (param "l" (list u8)) (result u32)
    (canon lift (core func $m "f") (memory (core memory $m "mem"))
      (realloc (core func $m "realloc"))))
  (core func $f' (canon lower (func $f) (memory (core memory $m "mem"))))
  (core module $N
    (import "" "f" (func $f (param i32 i32) (result i32)))
    (func (export "run") (result i32)
      (call $f (i32.const 100) (i32.const 3))))
  (core instance $n (instantiate $N (with "" (instance
    (export "f" (func $f'))))))
  (func (export "run") (result u32) (canon lift (core func $n "run")))
)
(assert_trap (invoke "run") "cannot enter component instance")

;; calls that don't involve linear memory at all also trap, at least for now
(component
  (core module $M (func (export "f") (result i32) i32.const 1))
  (core instance $m (instantiate $M))
  (func $f (result u32) (canon lift (core func $m "f")))
  (core func $f' (canon lower (func $f)))
  (core module $N
    (import "" "f" (func $f (result i32)))
    (func (export "run") (result i32) (call $f)))
  (core instance $n (instantiate $N (with "" (instance
    (export "f" (func $f'))))))
  (func (export "run") (result u32) (canon lift (core func $n "run")))
)
(assert_trap (invoke "run") "cannot enter component instance")
