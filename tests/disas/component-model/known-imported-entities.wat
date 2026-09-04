;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "function"
;;! flags = "-C inlining=n -Wconcurrency-support=n"

;; Module `$M` defines a memory, global, and table and module `$N` imports them.
;; Each module is instantiated exactly once, and nothing else in the component
;; can get its hands on those entities, so both modules statically know that
;; `$N`'s imports are always `$M`'s definitions. Both modules should therefore
;; use the precise `DefinedMemory`/`DefinedGlobal`/`DefinedTable` alias regions,
;; and use the same region as each other for the same entity, rather than
;; falling back to the conservative `PublicMemory`/`PublicGlobal`/`PublicTable`
;; regions.

(component
  (core module $M
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

  (core instance $m (instantiate $M))

  (core module $N
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

  (core instance $n (instantiate $N (with "" (instance $m))))
)
;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 215 ""
;;     region3 = 105 ""
;;     region4 = 171 ""
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
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 250 ""
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
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 93 ""
;;     region3 = 211 ""
;;     region4 = 117 ""
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
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 13 ""
;;     region3 = 215 ""
;;     region4 = 105 ""
;;     region5 = 171 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @00f5                               v4 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @00f5                               v5 = load.i64 notrap aligned readonly can_move region3 v4
;; @00f5                               v7 = load.i32 little region5 v5
;; @00f8                               jump block1
;;
;;                                 block1:
;; @00f8                               return v7
;; }
;;
;; function u1:1(i64 vmctx, i64) -> i32 tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 44 ""
;;     region3 = 250 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @00fb                               v2 = load.i64 notrap aligned readonly can_move region2 v0+96
;; @00fb                               v3 = load.i32 notrap aligned region3 v2
;; @00fd                               jump block1
;;
;;                                 block1:
;; @00fd                               return v3
;; }
;;
;; function u1:2(i64 vmctx, i64) -> i64 tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 21 ""
;;     region3 = 93 ""
;;     region4 = 211 ""
;;     region5 = 117 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0102                               v3 = load.i64 notrap aligned readonly can_move region2 v0+72
;; @0102                               v4 = load.i64 notrap aligned region4 v3+8
;; @0102                               v9 = load.i64 notrap aligned region3 v3
;; @0102                               v5 = ireduce.i32 v4
;; @0100                               v2 = iconst.i32 0
;;                                     v23 = icmp eq v5, v2  ; v2 = 0
;;                                     v26 = iconst.i64 0
;; @0102                               v14 = select_spectre_guard v23, v26, v9  ; v26 = 0
;; @0102                               v15 = load.i64 user6 aligned region5 v14
;; @0102                               v16 = iconst.i64 -2
;; @0102                               v17 = band v15, v16  ; v16 = -2
;; @0102                               brif v15, block3(v17), block2
;;
;;                                 block2 cold:
;;                                     v27 = iconst.i32 0
;;                                     v28 = iconst.i64 0
;; @0102                               v21 = call fn0(v0, v27, v28)  ; v27 = 0, v28 = 0
;; @0102                               jump block3(v21)
;;
;;                                 block3(v18: i64):
;; @0104                               jump block1
;;
;;                                 block1:
;; @0104                               return v18
;; }
