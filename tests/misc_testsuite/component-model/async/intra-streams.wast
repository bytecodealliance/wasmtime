;;! component_model_async = true
;;! component_model_more_async_builtins = true
;;! multi_memory = true

(component
  (core module $libc
     (func (export "realloc") (param i32 i32 i32 i32) (result i32)
       (i32.const 8)
     )
     (memory (export "m") 1)
  )
  (core instance $libc (instantiate $libc))

  (type $s (stream string))
  (core func $stream.new (canon stream.new $s))
  (core func $stream.read (canon stream.read $s async (memory $libc "m") (realloc (func $libc "realloc"))))
  (core func $stream.write (canon stream.write $s async (memory $libc "m")))

  (core module $m
    (import "" "m" (memory 1))
    (import "" "stream.new" (func $stream.new (result i64)))
    (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
    (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))

    (func (export "run")
      (local $tmp i64)
      (local $r i32)
      (local $w i32)
      (local.set $tmp (call $stream.new))

      (local.set $r (i32.wrap_i64 (local.get $tmp)))
      (local.set $w (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))

      (call $stream.read (local.get $r) (i32.const 0) (i32.const 1))
      i32.const -1 ;; BLOCKED
      i32.ne
      if unreachable end

      (call $stream.write (local.get $w) (i32.const 0) (i32.const 1))
      drop
    )
  )

  (core instance $i (instantiate $m
    (with "" (instance
      (export "m" (memory $libc "m"))
      (export "stream.new" (func $stream.new))
      (export "stream.read" (func $stream.read))
      (export "stream.write" (func $stream.write))
    ))
  ))

  (func (export "run") (canon lift (core func $i "run")))
)

(assert_trap (invoke "run") "cannot read from and write to intra-component future/stream with non-numeric payload")

;; intra-component u64 works
(component
  (core module $libc
     (memory (export "m") 1)
  )
  (core instance $libc (instantiate $libc))

  (type $s (stream u64))
  (core func $stream.new (canon stream.new $s))
  (core func $stream.read (canon stream.read $s async (memory $libc "m")))
  (core func $stream.write (canon stream.write $s async (memory $libc "m")))

  (core module $m
    (import "" "m" (memory 1))
    (import "" "stream.new" (func $stream.new (result i64)))
    (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
    (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))

    (func (export "run")
      (local $tmp i64)
      (local $r i32)
      (local $w i32)
      (local.set $tmp (call $stream.new))

      (local.set $r (i32.wrap_i64 (local.get $tmp)))
      (local.set $w (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))

      (call $stream.read (local.get $r) (i32.const 0) (i32.const 1))
      i32.const -1 ;; BLOCKED
      i32.ne
      if unreachable end

      (i64.store (i32.const 8) (i64.const 100))
      (call $stream.write (local.get $w) (i32.const 8) (i32.const 1))
      i32.const 0x10 ;; (1 << 4) | COMPLETED
      i32.ne
      if unreachable end

      (i64.load (i32.const 0))
      i64.const 100
      i64.ne
      if unreachable end
    )
  )

  (core instance $i (instantiate $m
    (with "" (instance
      (export "m" (memory $libc "m"))
      (export "stream.new" (func $stream.new))
      (export "stream.read" (func $stream.read))
      (export "stream.write" (func $stream.write))
    ))
  ))

  (func (export "run") (canon lift (core func $i "run")))
)

(assert_return (invoke "run"))

;; intra-component u64 works, even across different linear memories.
(component
  (core module $libc
     (memory (export "m1") 1)
     (memory (export "m2") 2)
  )
  (core instance $libc (instantiate $libc))

  (type $s (stream u64))
  (core func $stream.new (canon stream.new $s))
  (core func $stream.read (canon stream.read $s async (memory $libc "m1")))
  (core func $stream.write (canon stream.write $s async (memory $libc "m2")))

  (core module $m
    (import "libc" "m1" (memory $m1 1))
    (import "libc" "m2" (memory $m2 2))
    (import "" "stream.new" (func $stream.new (result i64)))
    (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
    (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))

    (func (export "run")
      (local $tmp i64)
      (local $r i32)
      (local $w i32)
      (local.set $tmp (call $stream.new))

      (local.set $r (i32.wrap_i64 (local.get $tmp)))
      (local.set $w (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))

      (call $stream.read (local.get $r) (i32.const 0) (i32.const 1))
      i32.const -1 ;; BLOCKED
      i32.ne
      if unreachable end

      (i64.store $m2 (i32.const 65536) (i64.const 100))
      (call $stream.write (local.get $w) (i32.const 65536) (i32.const 1))
      i32.const 0x10 ;; (1 << 4) | COMPLETED
      i32.ne
      if unreachable end

      (i64.load $m1 (i32.const 0))
      i64.const 100
      i64.ne
      if unreachable end
    )
  )

  (core instance $i (instantiate $m
    (with "libc" (instance $libc))
    (with "" (instance
      (export "stream.new" (func $stream.new))
      (export "stream.read" (func $stream.read))
      (export "stream.write" (func $stream.write))
    ))
  ))

  (func (export "run") (canon lift (core func $i "run")))
)

(assert_return (invoke "run"))

;; intrinsics should pick the correct memory
(component
  (core module $libc
     (memory (export "m1") 1)
     (memory (export "m2") 2)
  )
  (core instance $libc (instantiate $libc))

  (type $s (stream u64))
  (core func $stream.new (canon stream.new $s))
  (core func $stream.read (canon stream.read $s async (memory $libc "m1")))
  (core func $stream.write (canon stream.write $s async (memory $libc "m2")))

  (core module $m
    (import "libc" "m1" (memory $m1 1))
    (import "libc" "m2" (memory $m2 2))
    (import "" "stream.new" (func $stream.new (result i64)))
    (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
    (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))

    (func (export "run")
      (local $tmp i64)
      (local $r i32)
      (local $w i32)
      (local.set $tmp (call $stream.new))

      (local.set $r (i32.wrap_i64 (local.get $tmp)))
      (local.set $w (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))

      (call $stream.read (local.get $r) (i32.const 0) (i32.const 1))
      i32.const -1 ;; BLOCKED
      i32.ne
      if unreachable end

      (i64.store $m1 (i32.const 8) (i64.const 101))
      (i64.store $m2 (i32.const 8) (i64.const 102))

      (call $stream.write (local.get $w) (i32.const 8) (i32.const 1))
      i32.const 0x10 ;; (1 << 4) | COMPLETED
      i32.ne
      if unreachable end

      (i64.load $m1 (i32.const 0))
      i64.const 102
      i64.ne
      if unreachable end
    )
  )

  (core instance $i (instantiate $m
    (with "libc" (instance $libc))
    (with "" (instance
      (export "stream.new" (func $stream.new))
      (export "stream.read" (func $stream.read))
      (export "stream.write" (func $stream.write))
    ))
  ))

  (func (export "run") (canon lift (core func $i "run")))
)

(assert_return (invoke "run"))
