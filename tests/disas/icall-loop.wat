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

;; function u0:0(i64 vmctx, i64, i32) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+72
;;     sig0 = (i64 vmctx, i64) -> i32 fast
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i64 system_v
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @002b                               v4 = iconst.i32 2
;; @002b                               v5 = icmp uge v2, v4  ; v4 = 2
;; @002b                               v10 = iconst.i64 0
;; @002b                               v6 = uextend.i64 v2
;;                                     v29 = iconst.i64 3
;; @002b                               v8 = ishl v6, v29  ; v29 = 3
;;                                     v30 = iconst.i64 -2
;; @002b                               v17 = load.i64 notrap aligned readonly v0+56
;; @002b                               v18 = load.i64 notrap aligned readonly v17+72
;; @002b                               v15 = iconst.i32 0
;; @002b                               v21 = load.i64 notrap aligned readonly v0+64
;; @002b                               v22 = load.i32 notrap aligned readonly v21
;; @0027                               jump block2
;;
;;                                 block2:
;; @002b                               v7 = load.i64 notrap aligned v0+72
;; @002b                               v9 = iadd v7, v8
;;                                     v31 = iconst.i64 0
;;                                     v32 = select_spectre_guard v5, v31, v9  ; v31 = 0
;; @002b                               v12 = load.i64 table_oob aligned table v32
;;                                     v33 = iconst.i64 -2
;;                                     v34 = band v12, v33  ; v33 = -2
;; @002b                               brif v12, block5(v34), block4
;;
;;                                 block4 cold:
;;                                     v35 = iconst.i32 0
;; @002b                               v19 = call_indirect.i64 sig1, v18(v0, v35, v2)  ; v35 = 0
;; @002b                               jump block5(v19)
;;
;;                                 block5(v14: i64):
;; @002b                               v23 = load.i32 icall_null aligned readonly v14+24
;; @002b                               v24 = icmp eq v23, v22
;; @002b                               brif v24, block7, block6
;;
;;                                 block6 cold:
;; @002b                               trap bad_sig
;;
;;                                 block7:
;; @002b                               v25 = load.i64 notrap aligned readonly v14+16
;; @002b                               v26 = load.i64 notrap aligned readonly v14+32
;; @002b                               v27 = call_indirect sig0, v25(v26, v0)
;; @002e                               jump block2
;; }
;;
;; function u0:1(i64 vmctx, i64) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+72
;;     sig0 = (i64 vmctx, i64) -> i32 fast
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i64 system_v
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;;                                     v38 = iconst.i64 8
;;                                     v29 = iconst.i64 -2
;; @0038                               v16 = load.i64 notrap aligned readonly v0+56
;; @0038                               v17 = load.i64 notrap aligned readonly v16+72
;; @0038                               v14 = iconst.i32 0
;; @0036                               v2 = iconst.i32 1
;; @0038                               v20 = load.i64 notrap aligned readonly v0+64
;; @0038                               v21 = load.i32 notrap aligned readonly v20
;; @0034                               jump block2
;;
;;                                 block2:
;; @0038                               v6 = load.i64 notrap aligned v0+72
;;                                     v39 = iconst.i64 8
;;                                     v40 = iadd v6, v39  ; v39 = 8
;; @0038                               v11 = load.i64 table_oob aligned table v40
;;                                     v41 = iconst.i64 -2
;;                                     v42 = band v11, v41  ; v41 = -2
;; @0038                               brif v11, block5(v42), block4
;;
;;                                 block4 cold:
;;                                     v43 = iconst.i32 0
;;                                     v44 = iconst.i32 1
;; @0038                               v18 = call_indirect.i64 sig1, v17(v0, v43, v44)  ; v43 = 0, v44 = 1
;; @0038                               jump block5(v18)
;;
;;                                 block5(v13: i64):
;; @0038                               v22 = load.i32 icall_null aligned readonly v13+24
;; @0038                               v23 = icmp eq v22, v21
;; @0038                               brif v23, block7, block6
;;
;;                                 block6 cold:
;; @0038                               trap bad_sig
;;
;;                                 block7:
;; @0038                               v24 = load.i64 notrap aligned readonly v13+16
;; @0038                               v25 = load.i64 notrap aligned readonly v13+32
;; @0038                               v26 = call_indirect sig0, v24(v25, v0)
;; @003b                               jump block2
;; }
