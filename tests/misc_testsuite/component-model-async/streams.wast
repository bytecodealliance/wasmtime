;;! component_model_async = true

;; stream.new
(component
  (core module $m
    (import "" "stream.new" (func $stream-new (result i32)))
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

;; stream.cancel-read
(component
  (core module $m
    (import "" "stream.cancel-read" (func $stream-cancel-read (param i32) (result i32)))
  )
  (type $stream-type (stream u8))
  (core func $stream-cancel-read (canon stream.cancel-read $stream-type async))
  (core instance $i (instantiate $m (with "" (instance (export "stream.cancel-read" (func $stream-cancel-read))))))
)

;; stream.cancel-write
(component
  (core module $m
    (import "" "stream.cancel-write" (func $stream-cancel-write (param i32) (result i32)))
  )
  (type $stream-type (stream u8))
  (core func $stream-cancel-write (canon stream.cancel-write $stream-type async))
  (core instance $i (instantiate $m (with "" (instance (export "stream.cancel-write" (func $stream-cancel-write))))))
)

;; stream.close-readable
(component
  (core module $m
    (import "" "stream.close-readable" (func $stream-close-readable (param i32)))
  )
  (type $stream-type (stream u8))
  (core func $stream-close-readable (canon stream.close-readable $stream-type))
  (core instance $i (instantiate $m (with "" (instance (export "stream.close-readable" (func $stream-close-readable))))))
)

;; stream.close-writable
(component
  (core module $m
    (import "" "stream.close-writable" (func $stream-close-writable (param i32 i32)))
  )
  (type $stream-type (stream u8))
  (core func $stream-close-writable (canon stream.close-writable $stream-type))
  (core instance $i (instantiate $m (with "" (instance (export "stream.close-writable" (func $stream-close-writable))))))
)
