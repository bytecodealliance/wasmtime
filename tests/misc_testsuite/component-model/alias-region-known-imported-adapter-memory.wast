;;! multi_memory = true

;; Component `$A` exports a function returning a tuple, and component `$B`
;; imports it and lowers it with `$Mem`'s memory and realloc. The fused adapter
;; copies the returned tuple into `$Mem`'s memory, and `$N` -- which imports
;; that same memory -- reads the result back out. Every access of `$Mem`'s
;; memory (in `$Mem` itself, in `$N`, and in the adapter) must use the same
;; `DefinedMemory` alias region.
;;
;; When inlining is enabled (randomly by the wast file fuzzer) we shouldn't get
;; any stale values from alias analysis forwarding stored values to loads across
;; mismatched alias regions.

(component
  (component $A
    (core module $M
      (memory (export "mem") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (i32.const 0))
      (func (export "f") (param i32) (result i32)
        (i32.store (i32.const 8) (local.get 0))
        (i32.store offset=4 (i32.const 8) (local.get 0))
        (i32.const 8))
    )
    (core instance $m (instantiate $M))
    (func (export "f") (param "a" u32) (result (tuple u32 u32))
      (canon lift (core func $m "f")
        (memory $m "mem")
        (realloc (func $m "realloc"))))
  )

  (instance $a (instantiate $A))

  (component $B
    (import "f" (func $f (param "a" u32) (result (tuple u32 u32))))

    (core module $Mem
      (memory (export "mem") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (i32.const 0))
      (func (export "set-mem")
        (i32.store (i32.const 0) (i32.const 0x1234)))
    )
    (core instance $mem (instantiate $Mem))

    (core func $f' (canon lower (func $f)
      (memory $mem "mem")
      (realloc (func $mem "realloc"))))

    (core module $N
      (import "" "mem" (memory 1))
      (import "" "f'" (func $f' (param i32 i32)))
      (import "" "set-mem" (func $set-mem))

      (func (export "g") (result i32)
        (call $f' (i32.const 42) (i32.const 0))
        (i32.add (i32.load (i32.const 0))
                 (i32.load offset=4 (i32.const 0))))

      (func (export "probe-mem") (result i32)
        (i32.store (i32.const 0) (i32.const 1))
        (call $set-mem)
        (i32.load (i32.const 0)))
    )
    (core instance $n (instantiate $N
      (with "" (instance
        (export "mem" (memory $mem "mem"))
        (export "f'" (func $f'))
        (export "set-mem" (func $mem "set-mem"))
      ))
    ))

    (func (export "g") (result u32)
      (canon lift (core func $n "g")))
    (func (export "probe-mem") (result u32)
      (canon lift (core func $n "probe-mem")))
  )

  (instance $b (instantiate $B (with "f" (func $a "f"))))

  (export "g" (func $b "g"))
  (export "probe-mem" (func $b "probe-mem"))
)

(assert_return (invoke "g") (u32.const 84))
(assert_return (invoke "probe-mem") (u32.const 0x1234))
