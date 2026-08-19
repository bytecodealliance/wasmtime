;;! gc = true

;; `$M` defines a memory, global, and table; `$N` imports them and re-exports
;; them under the same names; and `$Q` imports them transitively, through `$N`'s
;; re-export. Every module is instantiated exactly once and nothing else can get
;; its hands on these entities, so the transitive import is still unambiguous
;; and all three modules must use the same precise
;; `DefinedMemory`/`DefinedGlobal`/`DefinedTable` region.
;;
;; When inlining is enabled (randomly by the wast file fuzzer) we shouldn't get
;; any stale values from alias analysis forwarding stored values to loads across
;; mismatched alias regions.

(component
  (core module $M
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

  (core instance $m (instantiate $M))

  (core module $N
    (import "" "mem" (memory 1))
    (import "" "g" (global (mut i32)))
    (import "" "t" (table 1 (ref i31)))

    ;; Re-export our imports.
    (export "mem" (memory 0))
    (export "g" (global 0))
    (export "t" (table 0))
  )

  (core instance $n (instantiate $N (with "" (instance $m))))

  ;; The observer: entities laundered through `$N`'s re-export, setters straight
  ;; from `$M`.
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
      (export "mem" (memory $n "mem"))
      (export "g" (global $n "g"))
      (export "t" (table $n "t"))
      (export "set-mem" (func $m "set-mem"))
      (export "set-global" (func $m "set-global"))
      (export "set-table" (func $m "set-table"))
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
