;;! component_model_async = true

;; stream.new
(component
  (core module $m
    (import "" "stream.new" (func $stream-new (result i64)))
  )
  (type $stream-type (stream u8))
  (core func $stream-new (canon stream.new $stream-type))
  (core instance $i (instantiate $m (with "" (instance (export "stream.new" (func $stream-new))))))
)

;; stream.read
(component
  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (core module $m
    (import "" "stream.read" (func $stream-read (param i32 i32 i32) (result i32)))
  )
  (type $stream-type (stream u8))
  (core func $stream-read (canon stream.read $stream-type async (memory $libc "memory")))
  (core instance $i (instantiate $m (with "" (instance (export "stream.read" (func $stream-read))))))
)

;; stream.read w/o memory
(component
  (core module $m
    (import "" "stream.read" (func $stream-read (param i32 i32 i32) (result i32)))
  )
  (type $stream-type (stream))
  (core func $stream-read (canon stream.read $stream-type async))
  (core instance $i (instantiate $m (with "" (instance (export "stream.read" (func $stream-read))))))
)

;; stream.read; with realloc
(component
  (core module $libc
    (func (export "realloc") (param i32 i32 i32 i32) (result i32) unreachable)
    (memory (export "memory") 1)
  )
  (core instance $libc (instantiate $libc))
  (core module $m
    (import "" "stream.read" (func $stream-read (param i32 i32 i32) (result i32)))
  )
  (type $stream-type (stream string))
  (core func $stream-read (canon stream.read $stream-type async (memory $libc "memory") (realloc (func $libc "realloc"))))
  (core instance $i (instantiate $m (with "" (instance (export "stream.read" (func $stream-read))))))
)

;; stream.write
(component
  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (core module $m
    (import "" "stream.write" (func $stream-write (param i32 i32 i32) (result i32)))
  )
  (type $stream-type (stream u8))
  (core func $stream-write (canon stream.write $stream-type async (memory $libc "memory")))
  (core instance $i (instantiate $m (with "" (instance (export "stream.write" (func $stream-write))))))
)

;; stream.write w/o memory
(component
  (core module $m
    (import "" "stream.write" (func $stream-write (param i32 i32 i32) (result i32)))
  )
  (type $stream-type (stream))
  (core func $stream-write (canon stream.write $stream-type async))
  (core instance $i (instantiate $m (with "" (instance (export "stream.write" (func $stream-write))))))
)

;; stream.cancel-read
(component
  (core module $m
    (import "" "stream.cancel-read" (func $stream-cancel-read (param i32) (result i32)))
  )
  (type $stream-type (stream u8))
  (core func $stream-cancel-read (canon stream.cancel-read $stream-type))
  (core instance $i (instantiate $m (with "" (instance (export "stream.cancel-read" (func $stream-cancel-read))))))
)

;; stream.cancel-write
(component
  (core module $m
    (import "" "stream.cancel-write" (func $stream-cancel-write (param i32) (result i32)))
  )
  (type $stream-type (stream u8))
  (core func $stream-cancel-write (canon stream.cancel-write $stream-type))
  (core instance $i (instantiate $m (with "" (instance (export "stream.cancel-write" (func $stream-cancel-write))))))
)

;; stream.drop-readable
(component
  (core module $m
    (import "" "stream.drop-readable" (func $stream-drop-readable (param i32)))
  )
  (type $stream-type (stream u8))
  (core func $stream-drop-readable (canon stream.drop-readable $stream-type))
  (core instance $i (instantiate $m (with "" (instance (export "stream.drop-readable" (func $stream-drop-readable))))))
)

;; stream.drop-writable
(component
  (core module $m
    (import "" "stream.drop-writable" (func $stream-drop-writable (param i32)))
  )
  (type $stream-type (stream u8))
  (core func $stream-drop-writable (canon stream.drop-writable $stream-type))
  (core instance $i (instantiate $m (with "" (instance (export "stream.drop-writable" (func $stream-drop-writable))))))
)

;; Test which exercises an intra-component stream read/write to ensure that
;; there's no Miri violations while doing this.
(component definition $A
  (core module $libc (memory (export "mem") 1))
  (core instance $libc (instantiate $libc))
  (core module $ics
    (import "" "stream.new" (func $stream.new (result i64)))
    (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
    (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))
    (import "" "mem" (memory 1))

    (global $r (mut i32) (i32.const 0))
    (global $w (mut i32) (i32.const 0))

    (func (export "read-twice")
      call $init

      (call $stream.read
        (global.get $r)
        (i32.const 100)
        (i32.const 100))
      i32.const -1 ;; BLOCKED
      i32.ne
      if unreachable end

      (call $stream.read
        (global.get $r)
        (i32.const 0)
        (i32.const 0))
      unreachable
    )

    (func (export "write-twice")
      call $init

      (call $stream.write
        (global.get $w)
        (i32.const 100)
        (i32.const 100))
      i32.const -1 ;; BLOCKED
      i32.ne
      if unreachable end

      (call $stream.write
        (global.get $w)
        (i32.const 0)
        (i32.const 0))
      unreachable
    )

    (func $init
      (local $t64 i64)

      (local.set $t64 (call $stream.new))
      (global.set $r (i32.wrap_i64 (local.get $t64)))
      (global.set $w (i32.wrap_i64 (i64.shr_u (local.get $t64) (i64.const 32))))
    )
  )
  (type $s (stream u8))
  (core func $stream.new (canon stream.new $s))
  (core func $stream.read (canon stream.read $s async (memory $libc "mem")))
  (core func $stream.write (canon stream.write $s async (memory $libc "mem")))
  (core instance $ics (instantiate $ics
    (with "" (instance
      (export "stream.new" (func $stream.new))
      (export "stream.read" (func $stream.read))
      (export "stream.write" (func $stream.write))
      (export "mem" (memory $libc "mem"))
    ))
  ))
  (func (export "read-twice") async (canon lift (core func $ics "read-twice")))
  (func (export "write-twice") async (canon lift (core func $ics "write-twice")))
)
(component instance $A $A)
(assert_trap (invoke "read-twice") "cannot have concurrent operations active on a future/stream")
(component instance $A $A)
(assert_trap (invoke "write-twice") "cannot have concurrent operations active on a future/stream")
