;; `$M`'s memory is exported and handed to the canonical ABI (it is the memory
;; and realloc backing a `canon lift` with a `string` parameter, so the string
;; transcoding intrinsics write into it), but it is still unambiguous: every
;; core module that can reach it knows exactly which memory it is. `$M` and the
;; importing module `$Q` must therefore agree on the precise `DefinedMemory`
;; alias region for it.
;;
;; When inlining is enabled (randomly by the wast file fuzzer) we shouldn't get
;; any stale values from alias analysis forwarding stored values to loads across
;; mismatched alias regions.

(component
  (core module $M
    (memory (export "mem") 1)

    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      (i32.const 16))

    ;; The core function behind the lifted `f` below: record the length of the
    ;; string the canonical ABI transcoded into our memory.
    (func (export "f") (param i32 i32)
      (i32.store (i32.const 8) (local.get 1)))

    (func (export "set-mem")
      (i32.store (i32.const 0) (i32.const 0x1234)))

    (func (export "get-len") (result i32)
      (i32.load (i32.const 8)))
  )

  (core instance $m (instantiate $M))

  (core module $Q
    (import "" "mem" (memory 1))
    (import "" "set-mem" (func $set-mem))

    (func (export "probe-mem") (result i32)
      (i32.store (i32.const 0) (i32.const 1))
      (call $set-mem)
      (i32.load (i32.const 0)))
  )

  (core instance $q (instantiate $Q (with "" (instance $m))))

  (func (export "f") (param "s" string)
    (canon lift (core func $m "f")
      (memory $m "mem")
      (realloc (func $m "realloc"))))

  (func (export "get-len") (result u32)
    (canon lift (core func $m "get-len")))

  (func (export "probe-mem") (result u32)
    (canon lift (core func $q "probe-mem")))
)

(assert_return (invoke "f" (str.const "hello")))
(assert_return (invoke "get-len") (u32.const 5))
(assert_return (invoke "probe-mem") (u32.const 0x1234))
