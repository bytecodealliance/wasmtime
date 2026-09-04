;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "function"
;;! flags = "-C inlining=n -Wconcurrency-support=n"

;; Same as `reexported-known-entities.wat` except that instead of a third module
;; defined inside this component, it is a core module *imported* by this
;; component that receives `$N`'s re-exports.
;;
;; Both `$M` and `$N` are still instantiated exactly once, but the imported
;; module is compiled separately from this component and so always accesses the
;; memory, global, and table it is given via the conservative `PublicMemory` /
;; `PublicGlobal`/`PublicTable` regions. Therefore `$M` and `$N` must use those
;; same conservative regions as well; using the precise `DefinedMemory` /
;; `DefinedGlobal`/`DefinedTable` regions here would mean the same entity is
;; accessed through two different alias regions, which is invalid.

(component
  (import "dyn" (core module $Dyn
    (import "" "mem" (memory 1))
    (import "" "g" (global (mut i32)))
    (import "" "t" (table 1 funcref))
  ))

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

  (core instance $n (instantiate $N (with "" (instance $m))))

  ;; Hand `$M`'s entities, laundered through `$N`'s re-export, to a module we
  ;; cannot see inside of.
  (core instance $d (instantiate $Dyn (with "" (instance $n))))
)
;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 215 ""
;;     region3 = 105 ""
;;     region4 = 61 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @009b                               v4 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @009b                               v6 = load.i32 little region4 v4
;; @009e                               jump block1
;;
;;                                 block1:
;; @009e                               return v6
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 87 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @00a1                               v2 = load.i32 notrap aligned region2 v0+96
;; @00a3                               jump block1
;;
;;                                 block1:
;; @00a3                               return v2
;; }
;;
;; function u0:2(i64 vmctx, i64) -> i64 tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 93 ""
;;     region3 = 211 ""
;;     region4 = 99 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @00a8                               v3 = load.i64 notrap aligned region3 v0+80
;; @00a8                               v7 = load.i64 notrap aligned region2 v0+72
;; @00a8                               v4 = ireduce.i32 v3
;; @00a6                               v2 = iconst.i32 0
;;                                     v21 = icmp eq v4, v2  ; v2 = 0
;;                                     v24 = iconst.i64 0
;; @00a8                               v12 = select_spectre_guard v21, v24, v7  ; v24 = 0
;; @00a8                               v13 = load.i64 user6 aligned region4 v12
;; @00a8                               v14 = iconst.i64 -2
;; @00a8                               v15 = band v13, v14  ; v14 = -2
;; @00a8                               brif v13, block3(v15), block2
;;
;;                                 block2 cold:
;;                                     v25 = iconst.i32 0
;;                                     v26 = iconst.i64 0
;; @00a8                               v19 = call fn0(v0, v25, v26)  ; v25 = 0, v26 = 0
;; @00a8                               jump block3(v19)
;;
;;                                 block3(v16: i64):
;; @00aa                               jump block1
;;
;;                                 block1:
;; @00aa                               return v16
;; }
;;
;; function u1:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 13 ""
;;     region3 = 215 ""
;;     region4 = 105 ""
;;     region5 = 61 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @012c                               v4 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @012c                               v5 = load.i64 notrap aligned readonly can_move region3 v4
;; @012c                               v7 = load.i32 little region5 v5
;; @012f                               jump block1
;;
;;                                 block1:
;; @012f                               return v7
;; }
;;
;; function u1:1(i64 vmctx, i64) -> i32 tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 44 ""
;;     region3 = 87 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0132                               v2 = load.i64 notrap aligned readonly can_move region2 v0+96
;; @0132                               v3 = load.i32 notrap aligned region3 v2
;; @0134                               jump block1
;;
;;                                 block1:
;; @0134                               return v3
;; }
;;
;; function u1:2(i64 vmctx, i64) -> i64 tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 21 ""
;;     region3 = 93 ""
;;     region4 = 211 ""
;;     region5 = 99 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0139                               v3 = load.i64 notrap aligned readonly can_move region2 v0+72
;; @0139                               v4 = load.i64 notrap aligned region4 v3+8
;; @0139                               v9 = load.i64 notrap aligned region3 v3
;; @0139                               v5 = ireduce.i32 v4
;; @0137                               v2 = iconst.i32 0
;;                                     v23 = icmp eq v5, v2  ; v2 = 0
;;                                     v26 = iconst.i64 0
;; @0139                               v14 = select_spectre_guard v23, v26, v9  ; v26 = 0
;; @0139                               v15 = load.i64 user6 aligned region5 v14
;; @0139                               v16 = iconst.i64 -2
;; @0139                               v17 = band v15, v16  ; v16 = -2
;; @0139                               brif v15, block3(v17), block2
;;
;;                                 block2 cold:
;;                                     v27 = iconst.i32 0
;;                                     v28 = iconst.i64 0
;; @0139                               v21 = call fn0(v0, v27, v28)  ; v27 = 0, v28 = 0
;; @0139                               jump block3(v21)
;;
;;                                 block3(v18: i64):
;; @013b                               jump block1
;;
;;                                 block1:
;; @013b                               return v18
;; }
