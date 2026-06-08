;;! target = "x86_64"
;;! test = "optimize"

;; When `call_indirect` is used in a loop with the same table index on every
;; iteration, we can hoist part of the work out of the loop. This test tracks
;; how much we're successfully pulling out.

(module
  (type $fn (func (result i32)))
  (table $fnptrs 2 2 funcref)
  (func (param i32)
        loop
          local.get 0
          call_indirect $fnptrs (type $fn)
          br 0
        end)
  (func
        loop
          i32.const 1
          call_indirect $fnptrs (type $fn)
          br 0
        end)
)

;; function u0:0(i64 vmctx, i64, i32) tail {
;;     region0 = 1342177280 "DefinedTable(StaticModuleIndex(0), DefinedTableIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @002b                               v4 = iconst.i32 2
;; @002b                               v5 = icmp uge v2, v4  ; v4 = 2
;; @002b                               v11 = iconst.i64 0
;; @002b                               v7 = load.i64 notrap aligned readonly can_move v0+48
;; @002b                               v6 = uextend.i64 v2
;; @002b                               v8 = iconst.i64 3
;; @002b                               v9 = ishl v6, v8  ; v8 = 3
;; @002b                               v10 = iadd v7, v9
;; @002b                               v12 = select_spectre_guard v5, v11, v10  ; v11 = 0
;; @002b                               v14 = iconst.i64 -2
;; @002b                               v17 = iconst.i32 0
;; @002b                               v20 = load.i64 notrap aligned readonly can_move v0+40
;; @002b                               v21 = load.i32 notrap aligned readonly can_move v20
;; @0027                               jump block2
;;
;;                                 block2:
;; @002b                               v13 = load.i64 user6 aligned region0 v12
;;                                     v30 = iconst.i64 -2
;;                                     v31 = band v13, v30  ; v30 = -2
;; @002b                               brif v13, block5(v31), block4
;;
;;                                 block4 cold:
;;                                     v32 = iconst.i32 0
;; @002b                               v19 = call fn0(v0, v32, v6)  ; v32 = 0
;; @002b                               jump block5(v19)
;;
;;                                 block5(v16: i64):
;; @002b                               v22 = load.i32 user7 aligned readonly v16+16
;; @002b                               v23 = icmp eq v22, v21
;; @002b                               trapz v23, user8
;; @002b                               v25 = load.i64 notrap aligned readonly v16+8
;; @002b                               v26 = load.i64 notrap aligned readonly v16+24
;; @002b                               v27 = call_indirect sig0, v25(v26, v0)
;; @002e                               jump block2
;; }
;;
;; function u0:1(i64 vmctx, i64) tail {
;;     region0 = 1342177280 "DefinedTable(StaticModuleIndex(0), DefinedTableIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0038                               v6 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v35 = iconst.i64 8
;; @0038                               v9 = iadd v6, v35  ; v35 = 8
;; @0038                               v13 = iconst.i64 -2
;; @0038                               v16 = iconst.i32 0
;;                                     v34 = iconst.i64 1
;; @0038                               v19 = load.i64 notrap aligned readonly can_move v0+40
;; @0038                               v20 = load.i32 notrap aligned readonly can_move v19
;; @0034                               jump block2
;;
;;                                 block2:
;;                                     v36 = iadd.i64 v6, v35  ; v35 = 8
;; @0038                               v12 = load.i64 user6 aligned region0 v36
;;                                     v37 = iconst.i64 -2
;;                                     v38 = band v12, v37  ; v37 = -2
;; @0038                               brif v12, block5(v38), block4
;;
;;                                 block4 cold:
;;                                     v39 = iconst.i32 0
;;                                     v40 = iconst.i64 1
;; @0038                               v18 = call fn0(v0, v39, v40)  ; v39 = 0, v40 = 1
;; @0038                               jump block5(v18)
;;
;;                                 block5(v15: i64):
;; @0038                               v21 = load.i32 user7 aligned readonly v15+16
;; @0038                               v22 = icmp eq v21, v20
;; @0038                               trapz v22, user8
;; @0038                               v24 = load.i64 notrap aligned readonly v15+8
;; @0038                               v25 = load.i64 notrap aligned readonly v15+24
;; @0038                               v26 = call_indirect sig0, v24(v25, v0)
;; @003b                               jump block2
;; }
