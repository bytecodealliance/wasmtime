;;! gc = true

;; `$M1` defines a memory, global, and table; `$N` imports and re-exports them;
;; and `$P` is instantiated twice, once with `$N`'s re-exports (really `$M1`'s
;; definitions) and once with `$M2`'s definitions directly. `$P` therefore
;; cannot statically know which entities it was handed, and that ambiguity has
;; to propagate back through `$N`'s re-export to `$M1` itself: every access of
;; `$M1`'s entities, in any module, must use the conservative
;; `PublicMemory`/`PublicGlobal`/`PublicTable` regions.
;;
;; When inlining is enabled (randomly by the wast file fuzzer) we shouldn't get
;; any stale values from alias analysis forwarding stored values to loads across
;; mismatched alias regions.

(component
  (core module $M1
    (memory (export "mem") 1)
    (global (export "g") (mut i32) (i32.const 0))
    (table (export "t") 1 (ref i31) (ref.i31 (i32.const 0)))

    (func (export "set-mem")
      (i32.store (i32.const 0) (i32.const 0x1234)))
    (func (export "set-global")
      (global.set 0 (i32.const 0x1234)))
    (func (export "set-table")
      (table.set 0 (i32.const 0) (ref.i31 (i32.const 0x1234))))
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
    (import "" "g" (global (mut i32)))
    (import "" "t" (table 1 (ref i31)))

    ;; Re-export our imports.
    (export "mem" (memory 0))
    (export "g" (global 0))
    (export "t" (table 0))
  )
  (core instance $n (instantiate $N (with "" (instance $m1))))

  ;; Instantiated twice, so its imports are not statically known and the
  ;; entities that flow into it -- including `$M1`'s, via `$N`'s re-export --
  ;; are ambiguous.
  (core module $P
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
  (core instance $p1 (instantiate $P (with "" (instance $n))))
  (core instance $p2 (instantiate $P (with "" (instance $m2))))

  (core module $Q
    (import "" "mem" (memory 1))
    (import "" "g" (global $g (mut i32)))
    (import "" "t" (table 1 (ref i31)))
    (import "" "m1-set-mem" (func $m1-set-mem))
    (import "" "m1-set-global" (func $m1-set-global))
    (import "" "m1-set-table" (func $m1-set-table))
    (import "" "p-set-mem" (func $p-set-mem))
    (import "" "p-set-global" (func $p-set-global))
    (import "" "p-set-table" (func $p-set-table))

    (func (export "probe-mem-via-m1") (result i32)
      (i32.store (i32.const 0) (i32.const 1))
      (call $m1-set-mem)
      (i32.load (i32.const 0)))
    (func (export "probe-global-via-m1") (result i32)
      (global.set $g (i32.const 1))
      (call $m1-set-global)
      (global.get $g))
    (func (export "probe-table-via-m1") (result i32)
      (local $before i32)
      (local.set $before (i31.get_u (table.get 0 (i32.const 0))))
      (call $m1-set-table)
      (i32.sub (i31.get_u (table.get 0 (i32.const 0))) (local.get $before)))

    (func (export "probe-mem-via-p") (result i32)
      (i32.store (i32.const 0) (i32.const 1))
      (call $p-set-mem)
      (i32.load (i32.const 0)))
    (func (export "probe-global-via-p") (result i32)
      (global.set $g (i32.const 1))
      (call $p-set-global)
      (global.get $g))
    (func (export "probe-table-via-p") (result i32)
      (local $before i32)
      (table.set 0 (i32.const 0) (ref.i31 (i32.const 0)))
      (local.set $before (i31.get_u (table.get 0 (i32.const 0))))
      (call $p-set-table)
      (i32.sub (i31.get_u (table.get 0 (i32.const 0))) (local.get $before)))
  )
  (core instance $q (instantiate $Q
    (with "" (instance
      (export "mem" (memory $n "mem"))
      (export "g" (global $n "g"))
      (export "t" (table $n "t"))
      (export "m1-set-mem" (func $m1 "set-mem"))
      (export "m1-set-global" (func $m1 "set-global"))
      (export "m1-set-table" (func $m1 "set-table"))
      (export "p-set-mem" (func $p1 "set-mem"))
      (export "p-set-global" (func $p1 "set-global"))
      (export "p-set-table" (func $p1 "set-table"))
    ))
  ))

  (func (export "probe-mem-via-m1") (result u32)
    (canon lift (core func $q "probe-mem-via-m1")))
  (func (export "probe-global-via-m1") (result u32)
    (canon lift (core func $q "probe-global-via-m1")))
  (func (export "probe-table-via-m1") (result u32)
    (canon lift (core func $q "probe-table-via-m1")))
  (func (export "probe-mem-via-p") (result u32)
    (canon lift (core func $q "probe-mem-via-p")))
  (func (export "probe-global-via-p") (result u32)
    (canon lift (core func $q "probe-global-via-p")))
  (func (export "probe-table-via-p") (result u32)
    (canon lift (core func $q "probe-table-via-p")))
)

(assert_return (invoke "probe-mem-via-m1") (u32.const 0x1234))
(assert_return (invoke "probe-global-via-m1") (u32.const 0x1234))
(assert_return (invoke "probe-table-via-m1") (u32.const 0x1234))
(assert_return (invoke "probe-mem-via-p") (u32.const 0x1234))
(assert_return (invoke "probe-global-via-p") (u32.const 0x1234))
(assert_return (invoke "probe-table-via-p") (u32.const 0x1234))
