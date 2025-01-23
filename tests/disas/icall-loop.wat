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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+72
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u1:9 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @002b                               v4 = iconst.i32 2
;; @002b                               v5 = icmp uge v2, v4  ; v4 = 2
;; @002b                               v10 = iconst.i64 0
;; @002b                               v7 = load.i64 notrap aligned readonly v0+72
;; @002b                               v6 = uextend.i64 v2
;;                                     v29 = iconst.i64 3
;; @002b                               v8 = ishl v6, v29  ; v29 = 3
;; @002b                               v9 = iadd v7, v8
;; @002b                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;;                                     v30 = iconst.i64 -2
;; @002b                               v15 = iconst.i32 0
;; @002b                               v20 = load.i64 notrap aligned readonly v0+64
;; @002b                               v21 = load.i32 notrap aligned readonly v20
;; @0027                               jump block2
;;
;;                                 block2:
;; @002b                               v12 = load.i64 user5 aligned table v11
;;                                     v31 = iconst.i64 -2
;;                                     v32 = band v12, v31  ; v31 = -2
;; @002b                               brif v12, block5(v32), block4
;;
;;                                 block4 cold:
;;                                     v33 = iconst.i32 0
;; @002b                               v18 = call fn0(v0, v33, v6)  ; v33 = 0
;; @002b                               jump block5(v18)
;;
;;                                 block5(v14: i64):
;; @002b                               v22 = load.i32 user6 aligned readonly v14+16
;; @002b                               v23 = icmp eq v22, v21
;; @002b                               trapz v23, user7
;; @002b                               v24 = load.i64 notrap aligned readonly v14+8
;; @002b                               v25 = load.i64 notrap aligned readonly v14+24
;; @002b                               v26 = call_indirect sig0, v24(v25, v0)
;; @002e                               jump block2
;; }
;;
;; function u0:1(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+72
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u1:9 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0038                               v6 = load.i64 notrap aligned readonly v0+72
;;                                     v37 = iconst.i64 8
;; @0038                               v8 = iadd v6, v37  ; v37 = 8
;;                                     v28 = iconst.i64 -2
;; @0038                               v14 = iconst.i32 0
;;                                     v36 = iconst.i64 1
;; @0038                               v19 = load.i64 notrap aligned readonly v0+64
;; @0038                               v20 = load.i32 notrap aligned readonly v19
;; @0034                               jump block2
;;
;;                                 block2:
;;                                     v38 = iadd.i64 v6, v37  ; v37 = 8
;; @0038                               v11 = load.i64 user5 aligned table v38
;;                                     v39 = iconst.i64 -2
;;                                     v40 = band v11, v39  ; v39 = -2
;; @0038                               brif v11, block5(v40), block4
;;
;;                                 block4 cold:
;;                                     v41 = iconst.i32 0
;;                                     v42 = iconst.i64 1
;; @0038                               v17 = call fn0(v0, v41, v42)  ; v41 = 0, v42 = 1
;; @0038                               jump block5(v17)
;;
;;                                 block5(v13: i64):
;; @0038                               v21 = load.i32 user6 aligned readonly v13+16
;; @0038                               v22 = icmp eq v21, v20
;; @0038                               trapz v22, user7
;; @0038                               v23 = load.i64 notrap aligned readonly v13+8
;; @0038                               v24 = load.i64 notrap aligned readonly v13+24
;; @0038                               v25 = call_indirect sig0, v23(v24, v0)
;; @003b                               jump block2
;; }
