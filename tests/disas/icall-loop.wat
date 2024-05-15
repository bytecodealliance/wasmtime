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
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, i64) -> i32 fast
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i64 system_v
;;     fn0 = colocated u1:9 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @002b                               v4 = iconst.i32 2
;; @002b                               v5 = icmp uge v2, v4  ; v4 = 2
;; @002b                               v10 = iconst.i64 0
;; @002b                               v7 = load.i64 notrap aligned readonly v0+88
;; @002b                               v6 = uextend.i64 v2
;;                                     v28 = iconst.i64 3
;; @002b                               v8 = ishl v6, v28  ; v28 = 3
;; @002b                               v9 = iadd v7, v8
;; @002b                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;;                                     v29 = iconst.i64 -2
;; @002b                               v15 = iconst.i32 0
;; @002b                               v19 = load.i64 notrap aligned readonly v0+80
;; @002b                               v20 = load.i32 notrap aligned readonly v19
;; @0027                               jump block2
;;
;;                                 block2:
;; @002b                               v12 = load.i64 table_oob aligned table v11
;;                                     v30 = iconst.i64 -2
;;                                     v31 = band v12, v30  ; v30 = -2
;; @002b                               brif v12, block5(v31), block4
;;
;;                                 block4 cold:
;;                                     v32 = iconst.i32 0
;; @002b                               v17 = call fn0(v0, v32, v2)  ; v32 = 0
;; @002b                               jump block5(v17)
;;
;;                                 block5(v14: i64):
;; @002b                               v21 = load.i32 icall_null aligned readonly v14+16
;; @002b                               v22 = icmp eq v21, v20
;; @002b                               brif v22, block7, block6
;;
;;                                 block6 cold:
;; @002b                               trap bad_sig
;;
;;                                 block7:
;; @002b                               v23 = load.i64 notrap aligned readonly v14+8
;; @002b                               v24 = load.i64 notrap aligned readonly v14+24
;; @002b                               v25 = call_indirect sig0, v23(v24, v0)
;; @002e                               jump block2
;; }
;;
;; function u0:1(i64 vmctx, i64) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, i64) -> i32 fast
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i64 system_v
;;     fn0 = colocated u1:9 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0038                               v6 = load.i64 notrap aligned readonly v0+88
;;                                     v36 = iconst.i64 8
;; @0038                               v8 = iadd v6, v36  ; v36 = 8
;;                                     v27 = iconst.i64 -2
;; @0038                               v14 = iconst.i32 0
;; @0036                               v2 = iconst.i32 1
;; @0038                               v18 = load.i64 notrap aligned readonly v0+80
;; @0038                               v19 = load.i32 notrap aligned readonly v18
;; @0034                               jump block2
;;
;;                                 block2:
;;                                     v37 = iadd.i64 v6, v36  ; v36 = 8
;; @0038                               v11 = load.i64 table_oob aligned table v37
;;                                     v38 = iconst.i64 -2
;;                                     v39 = band v11, v38  ; v38 = -2
;; @0038                               brif v11, block5(v39), block4
;;
;;                                 block4 cold:
;;                                     v40 = iconst.i32 0
;;                                     v41 = iconst.i32 1
;; @0038                               v16 = call fn0(v0, v40, v41)  ; v40 = 0, v41 = 1
;; @0038                               jump block5(v16)
;;
;;                                 block5(v13: i64):
;; @0038                               v20 = load.i32 icall_null aligned readonly v13+16
;; @0038                               v21 = icmp eq v20, v19
;; @0038                               brif v21, block7, block6
;;
;;                                 block6 cold:
;; @0038                               trap bad_sig
;;
;;                                 block7:
;; @0038                               v22 = load.i64 notrap aligned readonly v13+8
;; @0038                               v23 = load.i64 notrap aligned readonly v13+24
;; @0038                               v24 = call_indirect sig0, v22(v23, v0)
;; @003b                               jump block2
;; }
