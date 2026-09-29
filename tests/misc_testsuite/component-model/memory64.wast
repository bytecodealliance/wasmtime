;;! component_model_memory64 = true
;;! memory64 = true
;;! multi_memory = true
;;! bulk_memory = true
;;! hogs_memory = true

;; Exercise a fused adapter that passes a `string` across the boundary
;; between a 64-bit component (core memory is `i64`) and a 32-bit component
;; (core memory is `i32`).
;;
;; The top-level `roundtrip` export is lifted out of the 32-bit component. When
;; invoked, the string travels:
;;
;;   host -> (lift into c32's i32 memory)
;;        -> (lower out of c32's i32 memory into the canonical ABI)
;;        -> (lift into c64's i64 memory) -> c64 copies the bytes
;;        -> (lower back out of c64's i64 memory)
;;        -> (lift back into c32's i32 memory)
;;        -> host
;;
;; so the string is copied through both a 32-bit and a 64-bit linear memory in
;; both the argument and result directions.

(component
  (component $c64
    (core module $m
      ;; Just enough pages to be bigger than the max 32bit memory, and fit the data for our roundtrip
      (memory (export "memory") i64 0x1_0002 0x1_0005)

      ;; Start the allocator past the 4GB limit of 32bit memories
      (global $next (mut i64) (i64.const 4295032832)) ;; 0x1_0001 * 0x1_0000
      (func $realloc (export "realloc")
        (param $old i64) (param $old_sz i64) (param $align i64) (param $new_sz i64)
        (result i64)
        (local $ret i64)
        (local.set $ret
          (i64.and
            (i64.add (global.get $next) (i64.const 7))
            (i64.const -8)))
        (global.set $next (i64.add (local.get $ret) (local.get $new_sz)))
        (local.get $ret))

      (func (export "roundtrip") (param $ptr i64) (param $len i64) (result i64)
        (local $dst i64)
        (local $ret i64)

        (local.set $dst
          (call $realloc (i64.const 0) (i64.const 0) (i64.const 1) (local.get $len)))
        (memory.copy (local.get $dst) (local.get $ptr) (local.get $len))

        (local.set $ret
          (call $realloc (i64.const 0) (i64.const 0) (i64.const 8) (i64.const 16)))
        (i64.store (local.get $ret) (local.get $dst))
        (i64.store offset=8 (local.get $ret) (local.get $len))
        (local.get $ret))
    )
    (core instance $m (instantiate $m))

    (func (export "roundtrip") (param "a" string) (result string)
      (canon lift (core func $m "roundtrip")
        (memory (core memory $m "memory"))
        (realloc (core func $m "realloc"))))
  )
  (instance $c64 (instantiate $c64))

  (component $c32
    (import "backend" (instance $i
      (export "roundtrip" (func (param "a" string) (result string)))
    ))

    (core module $libc
      (memory (export "memory") 1)
      (global $next (mut i32) (i32.const 0))
      (func (export "realloc")
        (param $old i32) (param $old_sz i32) (param $align i32) (param $new_sz i32)
        (result i32)
        (local $ret i32)
        (local.set $ret
          (i32.and
            (i32.add (global.get $next) (i32.const 7))
            (i32.const -8)))
        (global.set $next (i32.add (local.get $ret) (local.get $new_sz)))
        (local.get $ret))
    )
    (core instance $libc (instantiate $libc))

    (core func $roundtrip
      (canon lower (func $i "roundtrip")
        (memory (core memory $libc "memory"))
        (realloc (core func $libc "realloc"))))

    (core module $m
      (import "" "memory" (memory 1))
      (import "" "realloc" (func $realloc (param i32 i32 i32 i32) (result i32)))
      (import "" "roundtrip" (func $roundtrip (param i32 i32 i32)))

      (func (export "roundtrip") (param $ptr i32) (param $len i32) (result i32)
        (local $ret i32)
        ;; Allocate space for the return value: [ptr:i32; len:i32]
        (local.set $ret (call $realloc (i32.const 0) (i32.const 0) (i32.const 1) (i32.const 8)))
        (call $roundtrip (local.get $ptr) (local.get $len) (local.get $ret))
        (local.get $ret))
    )
    (core instance $m (instantiate $m
      (with "" (instance
        (export "memory" (memory $libc "memory"))
        (export "realloc" (func $libc "realloc"))
        (export "roundtrip" (func $roundtrip))
      ))
    ))

    (func (export "roundtrip") (param "a" string) (result string)
      (canon lift (core func $m "roundtrip")
        (memory (core memory $libc "memory"))
        (realloc (core func $libc "realloc"))))
  )

  (instance $c32 (instantiate $c32 (with "backend" (instance $c64))))
  (export "roundtrip" (func $c32 "roundtrip"))
)

(assert_return
  (invoke "roundtrip" (str.const "hello"))
  (str.const "hello"))

(assert_return
  (invoke "roundtrip"
    (str.const "Hello, I'm a longer string asdljasdlkjasdlkjasdlkjasdljkasd0"))
  (str.const "Hello, I'm a longer string asdljasdlkjasdlkjasdlkjasdljkasd0"))

(assert_return
  (invoke "roundtrip" (str.const ""))
  (str.const ""))
