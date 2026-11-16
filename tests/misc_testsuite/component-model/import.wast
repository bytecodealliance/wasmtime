(assert_invalid
  (component
    (import "host-return-two" (func $f (result u32)))
    (export "x" (func $f)))
  "component export `x` is a reexport of an imported function which is not implemented")

(assert_unlinkable
  (component
    (import "host-return-two" (instance))
  )
  "expected instance found func")

;; empty instances don't need to be supplied by the host, even recursively
;; empty instances.
(component
  (import "not-provided-by-the-host" (instance))
  (import "not-provided-by-the-host2" (instance
    (export "x" (instance))
  ))
)

;; Lowering imports, with and without memory and realloc options, compiles.
(component definition
  (import "a" (func $f))
  (core func (canon lower (func $f)))
)
(component definition
  (import "a" (func $f (param "a" string)))
  (core module $libc
    (memory (export "memory") 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      unreachable)
  )
  (core instance $libc (instantiate $libc))
  (core func (canon lower (func $f)
    (memory (core memory $libc "memory"))
    (realloc (core func $libc "realloc"))))
)
(component definition
  (core module $libc
    (memory (export "memory") 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      unreachable)
  )

  (import "f1" (func $f1 (param "a" string) (result string)))
  (core instance $libc (instantiate $libc))
  (core func (canon lower (func $f1)
    (memory (core memory $libc "memory"))
    (realloc (core func $libc "realloc"))))

  (import "f2" (func $f2 (param "a" u32) (result (list u8))))
  (core instance $libc2 (instantiate $libc))
  (core func (canon lower (func $f2)
    (memory (core memory $libc2 "memory"))
    (realloc (core func $libc2 "realloc"))))

  (core func (canon lower (func $f1)
    (memory (core memory $libc2 "memory"))
    (realloc (core func $libc2 "realloc"))))
  (core func (canon lower (func $f2)
    (memory (core memory $libc "memory"))
    (realloc (core func $libc "realloc"))))
)
(component definition
  (import "log" (func $log (param "a" string)))
  (core module $libc
    (memory (export "memory") 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      unreachable)
  )
  (core instance $libc (instantiate $libc))
  (core func $log_lower (canon lower (func $log)
    (memory (core memory $libc "memory"))
    (realloc (core func $libc "realloc"))))

  (core module $logger
    (import "host" "log" (func $log (param i32 i32)))
    (import "libc" "memory" (memory 1))

    (func (export "call")
      i32.const 0
      i32.const 0
      call $log)
  )
  (core instance $logger (instantiate $logger
    (with "host" (instance (export "log" (func $log_lower))))
    (with "libc" (instance $libc))
  ))

  (func (export "call")
    (canon lift (core func $logger "call"))
  )
)
