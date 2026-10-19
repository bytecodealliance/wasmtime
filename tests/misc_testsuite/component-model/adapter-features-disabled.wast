;;! multi_memory = false

;; A string passed between two components needs an adapter that uses both
;; components' memories, which fails to validate without multi-memory.
(assert_invalid
  (component
    (component $A
      (core module $m
        (memory (export "memory") 1)
        (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 16)
        (func (export "f") (param i32 i32) (result i32) local.get 1))
      (core instance $i (instantiate $m))
      (func (export "f") (param "s" string) (result u32)
        (canon lift (core func $i "f") (memory (core memory $i "memory"))
          (realloc (core func $i "realloc")))))
    (component $B
      (import "f" (func $f (param "s" string) (result u32)))
      (core module $libc (memory (export "memory") 1))
      (core instance $libc (instantiate $libc))
      (core func $f (canon lower (func $f) (memory (core memory $libc "memory"))))
      (core module $m
        (import "" "f" (func $f (param i32 i32) (result i32)))
        (func (export "run") (result i32) (call $f (i32.const 0) (i32.const 3))))
      (core instance $i (instantiate $m (with "" (instance (export "f" (func $f))))))
      (func (export "run") (result u32) (canon lift (core func $i "run"))))
    (instance $a (instantiate $A))
    (instance $b (instantiate $B (with "f" (func $a "f"))))
    (export "run" (func $b "run")))
  "multiple memories")
