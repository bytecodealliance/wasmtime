;;! gc = true

;; `$N` is instantiated twice, once with `$M1`'s entities and once with `$M2`'s,
;; so nothing can statically know which memory, global, or table `$N` is looking
;; at. That ambiguity is a property of the entities themselves, not just of
;; `$N`: `$M1` and `$M2` must fall back to the conservative
;; `PublicMemory`/`PublicGlobal`/`PublicTable` regions for their own definitions
;; too, because otherwise inlining one of their functions into one of `$N`'s
;; would access the same entity through two different alias regions.
;;
;; When inlining is enabled (randomly by the wast file fuzzer) we shouldn't get
;; any stale values from alias analysis forwarding stored values to loads across
;; mismatched alias regions.

(component
  (core module $M1
    (memory (export "mem") 1)
    (global (export "g") (mut i32) (i32.const 0))
    (table (export "t") 1 (ref i31) (ref.i31 (i32.const 0)))
  )
  (core instance $m1 (instantiate $M1))

  (core module $M2
    (memory (export "mem") 1)
    (global (export "g") (mut i32) (i32.const 0))
    (table (export "t") 1 (ref i31) (ref.i31 (i32.const 0)))
  )
  (core instance $m2 (instantiate $M2))

  (core module $N
    (import "" "mem" (memory 1))
    (import "" "g" (global $g (mut i32)))
    (import "" "t" (table 1 (ref i31)))

    (func (export "set-mem")
      (i32.store (i32.const 0) (i32.const 0x1234)))
    (func (export "set-global")
      (global.set $g (i32.const 0x1234)))
    (func (export "set-table")
      (table.set 0 (i32.const 0) (ref.i31 (i32.const 0x1234))))
  )
  (core instance $n1 (instantiate $N (with "" (instance $m1))))
  (core instance $n2 (instantiate $N (with "" (instance $m2))))

  ;; The observer: entities straight from `$M1`, setters from `$N`.
  (core module $Q
    (import "" "mem" (memory 1))
    (import "" "g" (global $g (mut i32)))
    (import "" "t" (table 1 (ref i31)))
    (import "" "set-mem" (func $set-mem))
    (import "" "set-global" (func $set-global))
    (import "" "set-table" (func $set-table))

    (func (export "probe-mem") (result i32)
      (i32.store (i32.const 0) (i32.const 1))
      (call $set-mem)
      (i32.load (i32.const 0)))

    (func (export "probe-global") (result i32)
      (global.set $g (i32.const 1))
      (call $set-global)
      (global.get $g))

    (func (export "probe-table") (result i32)
      (local $before i32)
      (local.set $before (i31.get_u (table.get 0 (i32.const 0))))
      (call $set-table)
      (i32.sub (i31.get_u (table.get 0 (i32.const 0))) (local.get $before)))
  )
  (core instance $q (instantiate $Q
    (with "" (instance
      (export "mem" (memory $m1 "mem"))
      (export "g" (global $m1 "g"))
      (export "t" (table $m1 "t"))
      (export "set-mem" (func $n1 "set-mem"))
      (export "set-global" (func $n1 "set-global"))
      (export "set-table" (func $n1 "set-table"))
    ))
  ))

  (func (export "probe-mem") (result u32)
    (canon lift (core func $q "probe-mem")))
  (func (export "probe-global") (result u32)
    (canon lift (core func $q "probe-global")))
  (func (export "probe-table") (result u32)
    (canon lift (core func $q "probe-table")))
)

(assert_return (invoke "probe-mem") (u32.const 0x1234))
(assert_return (invoke "probe-global") (u32.const 0x1234))
(assert_return (invoke "probe-table") (u32.const 0x1234))
