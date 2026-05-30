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
;; @002b                               v10 = iconst.i64 0
;; @002b                               v7 = load.i64 notrap aligned readonly can_move v0+48
;; @002b                               v6 = uextend.i64 v2
;;                                     v29 = iconst.i64 3
;; @002b                               v8 = ishl v6, v29  ; v29 = 3
;; @002b                               v9 = iadd v7, v8
;; @002b                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @002b                               v13 = iconst.i64 -2
;; @002b                               v16 = iconst.i32 0
;; @002b                               v21 = load.i64 notrap aligned readonly can_move v0+40
;; @002b                               v22 = load.i32 notrap aligned readonly can_move v21
;; @0027                               jump block2
;;
;;                                 block2:
;; @002b                               v12 = load.i64 user6 aligned region0 v11
;;                                     v31 = iconst.i64 -2
;;                                     v32 = band v12, v31  ; v31 = -2
;; @002b                               brif v12, block5(v32), block4
;;
;;                                 block4 cold:
;;                                     v33 = iconst.i32 0
;; @002b                               v19 = call fn0(v0, v33, v6)  ; v33 = 0
;; @002b                               jump block5(v19)
;;
;;                                 block5(v15: i64):
;; @002b                               v23 = load.i32 user7 aligned readonly v15+16
;; @002b                               v24 = icmp eq v23, v22
;; @002b                               trapz v24, user8
;; @002b                               v25 = load.i64 notrap aligned readonly v15+8
;; @002b                               v26 = load.i64 notrap aligned readonly v15+24
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
;;                                     v36 = iconst.i64 8
;; @0038                               v8 = iadd v6, v36  ; v36 = 8
;; @0038                               v12 = iconst.i64 -2
;; @0038                               v15 = iconst.i32 0
;;                                     v35 = iconst.i64 1
;; @0038                               v20 = load.i64 notrap aligned readonly can_move v0+40
;; @0038                               v21 = load.i32 notrap aligned readonly can_move v20
;; @0034                               jump block2
;;
;;                                 block2:
;;                                     v37 = iadd.i64 v6, v36  ; v36 = 8
;; @0038                               v11 = load.i64 user6 aligned region0 v37
;;                                     v38 = iconst.i64 -2
;;                                     v39 = band v11, v38  ; v38 = -2
;; @0038                               brif v11, block5(v39), block4
;;
;;                                 block4 cold:
;;                                     v40 = iconst.i32 0
;;                                     v41 = iconst.i64 1
;; @0038                               v18 = call fn0(v0, v40, v41)  ; v40 = 0, v41 = 1
;; @0038                               jump block5(v18)
;;
;;                                 block5(v14: i64):
;; @0038                               v22 = load.i32 user7 aligned readonly v14+16
;; @0038                               v23 = icmp eq v22, v21
;; @0038                               trapz v23, user8
;; @0038                               v24 = load.i64 notrap aligned readonly v14+8
;; @0038                               v25 = load.i64 notrap aligned readonly v14+24
;; @0038                               v26 = call_indirect sig0, v24(v25, v0)
;; @003b                               jump block2
;; }
