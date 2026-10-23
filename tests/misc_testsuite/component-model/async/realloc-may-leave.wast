;;! component_model_async = true
;;! component_model_error_context = true
;;! multi_memory = true

;; Per the canonical ABI, `realloc` always runs with `may_leave` cleared, so it
;; can't call imports or intrinsics. Each component below reaches `realloc` via
;; a different path, and in every case `realloc` calls the host's
;; `host-return-two`, which must trap.

;; host -> guest: lowering params into a sync-lifted export.
(component
  (import "host-return-two" (func $h (result u32)))
  (core func $h (canon lower (func $h)))
  (core module $Libc
    (import "" "h" (func $h (result i32)))
    (memory (export "mem") 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      (drop (call $h)) (i32.const 0)))
  (core instance $libc (instantiate $Libc (with "" (instance (export "h" (func $h))))))
  (core module $M (func (export "f") (param i32 i32)))
  (core instance $m (instantiate $M))
  (func (export "f") (param "s" string)
    (canon lift (core func $m "f")
      (memory (core memory $libc "mem")) (realloc (core func $libc "realloc"))))
)
(assert_trap (invoke "f" (str.const "x")) "cannot leave component instance")

;; host -> guest: lowering params into an async-lifted export.
(component
  (import "host-return-two" (func $h (result u32)))
  (core func $h (canon lower (func $h)))
  (core module $Libc
    (import "" "h" (func $h (result i32)))
    (memory (export "mem") 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      (drop (call $h)) (i32.const 0)))
  (core instance $libc (instantiate $Libc (with "" (instance (export "h" (func $h))))))
  (core module $M
    (func (export "f") (param i32 i32) (result i32) unreachable)
    (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
  (core instance $m (instantiate $M))
  (func (export "f") async (param "s" string)
    (canon lift (core func $m "f") async (callback (core func $m "cb"))
      (memory (core memory $libc "mem")) (realloc (core func $libc "realloc"))))
)
(assert_trap (invoke "f" (str.const "x")) "cannot leave component instance")

;; host import -> guest: lowering a host function's results.
(component
  (import "host-return-two" (func $h (result u32)))
  (import "host" (instance $host (export "return-hi" (func (result string)))))
  (alias export $host "return-hi" (func $hi))
  (core func $h (canon lower (func $h)))
  (core module $Libc
    (import "" "h" (func $h (result i32)))
    (memory (export "mem") 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      (drop (call $h)) (i32.const 0)))
  (core instance $libc (instantiate $Libc (with "" (instance (export "h" (func $h))))))
  (core func $hi (canon lower (func $hi)
    (memory (core memory $libc "mem")) (realloc (core func $libc "realloc"))))
  (core module $M
    (import "" "hi" (func $hi (param i32)))
    (func (export "run") (call $hi (i32.const 8))))
  (core instance $m (instantiate $M (with "" (instance (export "hi" (func $hi))))))
  (func (export "run") (canon lift (core func $m "run")))
)
(assert_trap (invoke "run") "cannot leave component instance")

;; guest -> guest: copying a `future<string>` value into the reader. The writer
;; writes first, so the reader's `future.read` completes immediately.
(component
  (import "host-return-two" (func $h (result u32)))
  (component $C
    (import "host-return-two" (func $h (result u32)))
    (type $FT (future string))
    (core func $h (canon lower (func $h)))
    (core module $Libc
      (import "" "h" (func $h (result i32)))
      (memory (export "mem") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (drop (call $h)) (i32.const 0)))
    (core instance $libc (instantiate $Libc (with "" (instance (export "h" (func $h))))))
    (core func $future.read (canon future.read $FT async
      (memory (core memory $libc "mem")) (realloc (core func $libc "realloc"))))
    (core module $M
      (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
      (func (export "read") (param i32) (drop (call $future.read (local.get 0) (i32.const 8)))))
    (core instance $m (instantiate $M (with "" (instance
      (export "future.read" (func $future.read))))))
    (func (export "read") (param "f" (future string)) (canon lift (core func $m "read")))
  )
  (component $D
    (import "read" (func $read (param "f" (future string))))
    (type $FT (future string))
    (core module $Mem
      (memory (export "mem") 1)
      (data (i32.const 4) "\01")) ;; string at 0: ptr = 0, len = 1
    (core instance $mem (instantiate $Mem))
    (core func $read (canon lower (func $read)))
    (core func $future.new (canon future.new $FT))
    (core func $future.write (canon future.write $FT async (memory (core memory $mem "mem"))))
    (core module $M
      (import "" "read" (func $read (param i32)))
      (import "" "future.new" (func $future.new (result i64)))
      (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
      (func (export "run")
        (drop (call $future.new)) ;; readable = 1, writable = 2
        (drop (call $future.write (i32.const 2) (i32.const 0)))
        (call $read (i32.const 1))))
    (core instance $m (instantiate $M (with "" (instance
      (export "read" (func $read))
      (export "future.new" (func $future.new))
      (export "future.write" (func $future.write))))))
    (func (export "run") (canon lift (core func $m "run")))
  )
  (instance $c (instantiate $C (with "host-return-two" (func $h))))
  (instance $d (instantiate $D (with "read" (func $c "read"))))
  (export "run" (func $d "run"))
)
(assert_trap (invoke "run") "cannot leave component instance")

;; guest -> guest: copying a `stream<string>` element into the reader. The
;; writer writes first, so the reader's `stream.read` completes immediately.
(component
  (import "host-return-two" (func $h (result u32)))
  (component $C
    (import "host-return-two" (func $h (result u32)))
    (type $ST (stream string))
    (core func $h (canon lower (func $h)))
    (core module $Libc
      (import "" "h" (func $h (result i32)))
      (memory (export "mem") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (drop (call $h)) (i32.const 0)))
    (core instance $libc (instantiate $Libc (with "" (instance (export "h" (func $h))))))
    (core func $stream.read (canon stream.read $ST async
      (memory (core memory $libc "mem")) (realloc (core func $libc "realloc"))))
    (core module $M
      (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
      (func (export "read") (param i32)
        (drop (call $stream.read (local.get 0) (i32.const 8) (i32.const 1)))))
    (core instance $m (instantiate $M (with "" (instance
      (export "stream.read" (func $stream.read))))))
    (func (export "read") (param "s" (stream string)) (canon lift (core func $m "read")))
  )
  (component $D
    (import "read" (func $read (param "s" (stream string))))
    (type $ST (stream string))
    (core module $Mem
      (memory (export "mem") 1)
      (data (i32.const 4) "\01")) ;; string at 0: ptr = 0, len = 1
    (core instance $mem (instantiate $Mem))
    (core func $read (canon lower (func $read)))
    (core func $stream.new (canon stream.new $ST))
    (core func $stream.write (canon stream.write $ST async (memory (core memory $mem "mem"))))
    (core module $M
      (import "" "read" (func $read (param i32)))
      (import "" "stream.new" (func $stream.new (result i64)))
      (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))
      (func (export "run")
        (drop (call $stream.new)) ;; readable = 1, writable = 2
        (drop (call $stream.write (i32.const 2) (i32.const 0) (i32.const 1)))
        (call $read (i32.const 1))))
    (core instance $m (instantiate $M (with "" (instance
      (export "read" (func $read))
      (export "stream.new" (func $stream.new))
      (export "stream.write" (func $stream.write))))))
    (func (export "run") (canon lift (core func $m "run")))
  )
  (instance $c (instantiate $C (with "host-return-two" (func $h))))
  (instance $d (instantiate $D (with "read" (func $c "read"))))
  (export "run" (func $d "run"))
)
(assert_trap (invoke "run") "cannot leave component instance")

;; `error-context.debug-message`: lowering the message string into the caller.
(component
  (import "host-return-two" (func $h (result u32)))
  (core func $h (canon lower (func $h)))
  (core module $Libc
    (import "" "h" (func $h (result i32)))
    (memory (export "mem") 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      (drop (call $h)) (i32.const 0)))
  (core instance $libc (instantiate $Libc (with "" (instance (export "h" (func $h))))))
  (core func $error-context.new (canon error-context.new (memory (core memory $libc "mem"))))
  (core func $error-context.debug-message (canon error-context.debug-message
    (memory (core memory $libc "mem")) (realloc (core func $libc "realloc"))))
  (core module $M
    (import "" "error-context.new" (func $error-context.new (param i32 i32) (result i32)))
    (import "" "error-context.debug-message" (func $error-context.debug-message (param i32 i32)))
    (func (export "run")
      (call $error-context.debug-message
        (call $error-context.new (i32.const 0) (i32.const 1))
        (i32.const 8))))
  (core instance $m (instantiate $M (with "" (instance
    (export "error-context.new" (func $error-context.new))
    (export "error-context.debug-message" (func $error-context.debug-message))))))
  (func (export "run") (canon lift (core func $m "run")))
)
(assert_trap (invoke "run") "cannot leave component instance")
