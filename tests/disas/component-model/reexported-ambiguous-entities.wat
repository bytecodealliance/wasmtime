;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "function"
;;! flags = "-C inlining=n -Wconcurrency-support=n"

;; Same as `reexported-known-entities.wat` except that `$P` is instantiated
;; twice: once with `$N`'s re-exports (which are really `$M1`'s definitions) and
;; once with `$M2`'s definitions directly.
;;
;; `$N` itself is still instantiated exactly once, so looking only at `$N`'s
;; instantiation its imports appear unambiguous. But the entities it re-exports
;; flow onwards into `$P`, which cannot statically know which of `$M1`'s or
;; `$M2`'s entities it was handed. That ambiguity has to propagate back through
;; `$N`'s re-export to `$M1`, so every access of `$M1`'s memory, global, and
;; table -- in `$M1`, in `$N`, and in `$P` -- must use the conservative
;; `PublicMemory`/`PublicGlobal`/`PublicTable` regions, as must every access of
;; `$M2`'s. If `$M1` and `$N` kept the precise `DefinedMemory`/`DefinedGlobal`/
;; `DefinedTable` regions while `$P` used the public ones, then inlining one of
;; `$M1`'s or `$N`'s functions into one of `$P`'s would access the same entity
;; through two different alias regions, which is invalid.

(component
  (core module $M1
    (memory (export "mem") 1)
    (global (export "g") (mut i32) (i32.const 0))
    (table (export "t") 1 funcref)

    (func (export "load-mem") (result i32)
      (i32.load (i32.const 0)))
    (func (export "get-global") (result i32)
      (global.get 0))
    (func (export "get-table") (result funcref)
      (table.get 0 (i32.const 0)))
  )

  (core instance $m1 (instantiate $M1))

  (core module $M2
    (memory (export "mem") 1)
    (global (export "g") (mut i32) (i32.const 0))
    (table (export "t") 1 funcref)

    (func (export "load-mem") (result i32)
      (i32.load (i32.const 0)))
    (func (export "get-global") (result i32)
      (global.get 0))
    (func (export "get-table") (result funcref)
      (table.get 0 (i32.const 0)))
  )

  (core instance $m2 (instantiate $M2))

  (core module $N
    (import "" "mem" (memory 1))
    (import "" "g" (global (mut i32)))
    (import "" "t" (table 1 funcref))

    ;; Re-export our imports.
    (export "mem" (memory 0))
    (export "g" (global 0))
    (export "t" (table 0))

    (func (export "load-mem") (result i32)
      (i32.load (i32.const 0)))
    (func (export "get-global") (result i32)
      (global.get 0))
    (func (export "get-table") (result funcref)
      (table.get 0 (i32.const 0)))
  )

  (core instance $n (instantiate $N (with "" (instance $m1))))

  (core module $P
    (import "" "mem" (memory 1))
    (import "" "g" (global (mut i32)))
    (import "" "t" (table 1 funcref))

    (func (export "load-mem") (result i32)
      (i32.load (i32.const 0)))
    (func (export "get-global") (result i32)
      (global.get 0))
    (func (export "get-table") (result funcref)
      (table.get 0 (i32.const 0)))
  )

  ;; `$P` gets `$M1`'s entities via `$N`'s re-export here...
  (core instance $p1 (instantiate $P (with "" (instance $n))))
  ;; ...and `$M2`'s entities here.
  (core instance $p2 (instantiate $P (with "" (instance $m2))))
)
;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 15 "VMContext+0x8"
;;     region1 = 114 "VMStoreContext+0x18"
;;     region2 = 193 "VMMemoryDefinition+0x0"
;;     region3 = 213 "VMMemoryDefinition+0x8"
;;     region4 = 53 "PublicMemory"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0073                               v4 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @0073                               v6 = load.i32 little region4 v4
;; @0076                               jump block1
;;
;;                                 block1:
;; @0076                               return v6
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     region0 = 15 "VMContext+0x8"
;;     region1 = 114 "VMStoreContext+0x18"
;;     region2 = 172 "PublicGlobal"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0079                               v2 = load.i32 notrap aligned region2 v0+96
;; @007b                               jump block1
;;
;;                                 block1:
;; @007b                               return v2
;; }
;;
;; function u0:2(i64 vmctx, i64) -> i64 tail {
;;     region0 = 15 "VMContext+0x8"
;;     region1 = 114 "VMStoreContext+0x18"
;;     region2 = 44 "VMTableDefinition+0x0"
;;     region3 = 32 "VMTableDefinition+0x8"
;;     region4 = 156 "PublicTable"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0080                               v3 = load.i64 notrap aligned region3 v0+80
;; @0080                               v7 = load.i64 notrap aligned region2 v0+72
;; @0080                               v4 = ireduce.i32 v3
;; @007e                               v2 = iconst.i32 0
;;                                     v21 = icmp eq v4, v2  ; v2 = 0
;;                                     v24 = iconst.i64 0
;; @0080                               v12 = select_spectre_guard v21, v24, v7  ; v24 = 0
;; @0080                               v13 = load.i64 user6 aligned region4 v12
;; @0080                               v14 = iconst.i64 -2
;; @0080                               v15 = band v13, v14  ; v14 = -2
;; @0080                               brif v13, block3(v15), block2
;;
;;                                 block2 cold:
;;                                     v25 = iconst.i32 0
;;                                     v26 = iconst.i64 0
;; @0080                               v19 = call fn0(v0, v25, v26)  ; v25 = 0, v26 = 0
;; @0080                               jump block3(v19)
;;
;;                                 block3(v16: i64):
;; @0082                               jump block1
;;
;;                                 block1:
;; @0082                               return v16
;; }
;;
;; function u1:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 15 "VMContext+0x8"
;;     region1 = 114 "VMStoreContext+0x18"
;;     region2 = 193 "VMMemoryDefinition+0x0"
;;     region3 = 213 "VMMemoryDefinition+0x8"
;;     region4 = 53 "PublicMemory"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0100                               v4 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @0100                               v6 = load.i32 little region4 v4
;; @0103                               jump block1
;;
;;                                 block1:
;; @0103                               return v6
;; }
;;
;; function u1:1(i64 vmctx, i64) -> i32 tail {
;;     region0 = 15 "VMContext+0x8"
;;     region1 = 114 "VMStoreContext+0x18"
;;     region2 = 172 "PublicGlobal"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0106                               v2 = load.i32 notrap aligned region2 v0+96
;; @0108                               jump block1
;;
;;                                 block1:
;; @0108                               return v2
;; }
;;
;; function u1:2(i64 vmctx, i64) -> i64 tail {
;;     region0 = 15 "VMContext+0x8"
;;     region1 = 114 "VMStoreContext+0x18"
;;     region2 = 44 "VMTableDefinition+0x0"
;;     region3 = 32 "VMTableDefinition+0x8"
;;     region4 = 156 "PublicTable"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @010d                               v3 = load.i64 notrap aligned region3 v0+80
;; @010d                               v7 = load.i64 notrap aligned region2 v0+72
;; @010d                               v4 = ireduce.i32 v3
;; @010b                               v2 = iconst.i32 0
;;                                     v21 = icmp eq v4, v2  ; v2 = 0
;;                                     v24 = iconst.i64 0
;; @010d                               v12 = select_spectre_guard v21, v24, v7  ; v24 = 0
;; @010d                               v13 = load.i64 user6 aligned region4 v12
;; @010d                               v14 = iconst.i64 -2
;; @010d                               v15 = band v13, v14  ; v14 = -2
;; @010d                               brif v13, block3(v15), block2
;;
;;                                 block2 cold:
;;                                     v25 = iconst.i32 0
;;                                     v26 = iconst.i64 0
;; @010d                               v19 = call fn0(v0, v25, v26)  ; v25 = 0, v26 = 0
;; @010d                               jump block3(v19)
;;
;;                                 block3(v16: i64):
;; @010f                               jump block1
;;
;;                                 block1:
;; @010f                               return v16
;; }
;;
;; function u2:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 15 "VMContext+0x8"
;;     region1 = 114 "VMStoreContext+0x18"
;;     region2 = 112 "VMMemoryImport+0x0"
;;     region3 = 193 "VMMemoryDefinition+0x0"
;;     region4 = 213 "VMMemoryDefinition+0x8"
;;     region5 = 53 "PublicMemory"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0192                               v4 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @0192                               v5 = load.i64 notrap aligned readonly can_move region3 v4
;; @0192                               v7 = load.i32 little region5 v5
;; @0195                               jump block1
;;
;;                                 block1:
;; @0195                               return v7
;; }
;;
;; function u2:1(i64 vmctx, i64) -> i32 tail {
;;     region0 = 15 "VMContext+0x8"
;;     region1 = 114 "VMStoreContext+0x18"
;;     region2 = 190 "VMGlobalImport+0x0"
;;     region3 = 172 "PublicGlobal"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0198                               v2 = load.i64 notrap aligned readonly can_move region2 v0+96
;; @0198                               v3 = load.i32 notrap aligned region3 v2
;; @019a                               jump block1
;;
;;                                 block1:
;; @019a                               return v3
;; }
;;
;; function u2:2(i64 vmctx, i64) -> i64 tail {
;;     region0 = 15 "VMContext+0x8"
;;     region1 = 114 "VMStoreContext+0x18"
;;     region2 = 33 "VMTableImport+0x0"
;;     region3 = 44 "VMTableDefinition+0x0"
;;     region4 = 32 "VMTableDefinition+0x8"
;;     region5 = 156 "PublicTable"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @019f                               v3 = load.i64 notrap aligned readonly can_move region2 v0+72
;; @019f                               v4 = load.i64 notrap aligned region4 v3+8
;; @019f                               v9 = load.i64 notrap aligned region3 v3
;; @019f                               v5 = ireduce.i32 v4
;; @019d                               v2 = iconst.i32 0
;;                                     v23 = icmp eq v5, v2  ; v2 = 0
;;                                     v26 = iconst.i64 0
;; @019f                               v14 = select_spectre_guard v23, v26, v9  ; v26 = 0
;; @019f                               v15 = load.i64 user6 aligned region5 v14
;; @019f                               v16 = iconst.i64 -2
;; @019f                               v17 = band v15, v16  ; v16 = -2
;; @019f                               brif v15, block3(v17), block2
;;
;;                                 block2 cold:
;;                                     v27 = iconst.i32 0
;;                                     v28 = iconst.i64 0
;; @019f                               v21 = call fn0(v0, v27, v28)  ; v27 = 0, v28 = 0
;; @019f                               jump block3(v21)
;;
;;                                 block3(v18: i64):
;; @01a1                               jump block1
;;
;;                                 block1:
;; @01a1                               return v18
;; }
;;
;; function u3:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 15 "VMContext+0x8"
;;     region1 = 114 "VMStoreContext+0x18"
;;     region2 = 112 "VMMemoryImport+0x0"
;;     region3 = 193 "VMMemoryDefinition+0x0"
;;     region4 = 213 "VMMemoryDefinition+0x8"
;;     region5 = 53 "PublicMemory"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0217                               v4 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @0217                               v5 = load.i64 notrap aligned readonly can_move region3 v4
;; @0217                               v7 = load.i32 little region5 v5
;; @021a                               jump block1
;;
;;                                 block1:
;; @021a                               return v7
;; }
;;
;; function u3:1(i64 vmctx, i64) -> i32 tail {
;;     region0 = 15 "VMContext+0x8"
;;     region1 = 114 "VMStoreContext+0x18"
;;     region2 = 190 "VMGlobalImport+0x0"
;;     region3 = 172 "PublicGlobal"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @021d                               v2 = load.i64 notrap aligned readonly can_move region2 v0+96
;; @021d                               v3 = load.i32 notrap aligned region3 v2
;; @021f                               jump block1
;;
;;                                 block1:
;; @021f                               return v3
;; }
;;
;; function u3:2(i64 vmctx, i64) -> i64 tail {
;;     region0 = 15 "VMContext+0x8"
;;     region1 = 114 "VMStoreContext+0x18"
;;     region2 = 33 "VMTableImport+0x0"
;;     region3 = 44 "VMTableDefinition+0x0"
;;     region4 = 32 "VMTableDefinition+0x8"
;;     region5 = 156 "PublicTable"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0224                               v3 = load.i64 notrap aligned readonly can_move region2 v0+72
;; @0224                               v4 = load.i64 notrap aligned region4 v3+8
;; @0224                               v9 = load.i64 notrap aligned region3 v3
;; @0224                               v5 = ireduce.i32 v4
;; @0222                               v2 = iconst.i32 0
;;                                     v23 = icmp eq v5, v2  ; v2 = 0
;;                                     v26 = iconst.i64 0
;; @0224                               v14 = select_spectre_guard v23, v26, v9  ; v26 = 0
;; @0224                               v15 = load.i64 user6 aligned region5 v14
;; @0224                               v16 = iconst.i64 -2
;; @0224                               v17 = band v15, v16  ; v16 = -2
;; @0224                               brif v15, block3(v17), block2
;;
;;                                 block2 cold:
;;                                     v27 = iconst.i32 0
;;                                     v28 = iconst.i64 0
;; @0224                               v21 = call fn0(v0, v27, v28)  ; v27 = 0, v28 = 0
;; @0224                               jump block3(v21)
;;
;;                                 block3(v18: i64):
;; @0226                               jump block1
;;
;;                                 block1:
;; @0226                               return v18
;; }
