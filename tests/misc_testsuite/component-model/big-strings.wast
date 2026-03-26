;;! multi_memory = true
;;! hogs_memory = true

;; Sending a massive string
(component definition $A
  (component $A
    (core module $m
      (memory (export "m") 1)
      (func (export "f") (param i32 i32) unreachable)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32) unreachable)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "x" string)
      (canon lift
        (core func $i "f")
        (memory $i "m")
        (realloc (func $i "realloc"))
      )
    )
  )
  (instance $a (instantiate $A))

  (component $B
    (import "f" (func $f (param "x" string)))
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) (memory $libc "mem")))
    (core module $m
      (import "" "f" (func $f (param i32 i32)))
      (import "" "mem" (memory 1))

      (func (export "run") (param i32)
        (call $f (i32.const 0) (local.get 0)))

      (func (export "grow") (param i32) (result i32)
        (memory.grow (local.get 0)))
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "f" (func $f))
        (export "mem" (memory $libc "mem"))
      ))
    ))
    (func (export "run") (param "x" u32) (canon lift (core func $i "run")))
    (func (export "grow") (param "x" u32) (result s32)
      (canon lift (core func $i "grow")))
  )
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "run" (func $b "run"))
  (export "grow" (func $b "grow"))
)

;; Wildly out of bounds is just rejected
(component instance $A $A)
(assert_trap (invoke "run" (u32.const 0x8000_0000)) "string content out-of-bounds")

;; In-bounds, and just under the limit. Should hit the `unreachable` in the
;; `realloc`.
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "run" (u32.const 0x7fff_ffff)) "unreachable")

;; Size exceeds `(1<<31)-1`
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "run" (u32.const 0x8000_0000)) "string content out-of-bounds")
