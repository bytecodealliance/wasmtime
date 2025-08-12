;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (array (mut i64)))

  (func (param i64 i32) (result (ref $ty))
    (array.new $ty (local.get 0) (local.get 1))
  )
)
;; function u0:0(i64 vmctx, i64, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned gv4+32
;;     gv6 = load.i64 notrap aligned readonly can_move gv4+24
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     fn0 = colocated u1610612736:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0022                               v6 = uextend.i64 v3
;;                                     v55 = iconst.i64 3
;;                                     v56 = ishl v6, v55  ; v55 = 3
;;                                     v53 = iconst.i64 32
;; @0022                               v8 = ushr v56, v53  ; v53 = 32
;; @0022                               trapnz v8, user18
;; @0022                               v5 = iconst.i32 16
;;                                     v62 = iconst.i32 3
;;                                     v63 = ishl v3, v62  ; v62 = 3
;; @0022                               v10 = uadd_overflow_trap v5, v63, user18  ; v5 = 16
;; @0022                               v12 = iconst.i32 -67108864
;; @0022                               v13 = band v10, v12  ; v12 = -67108864
;; @0022                               trapnz v13, user18
;; @0022                               v15 = load.i64 notrap aligned readonly v0+32
;; @0022                               v16 = load.i32 notrap aligned v15
;;                                     v66 = iconst.i32 7
;; @0022                               v19 = uadd_overflow_trap v16, v66, user18  ; v66 = 7
;;                                     v73 = iconst.i32 -8
;; @0022                               v21 = band v19, v73  ; v73 = -8
;; @0022                               v22 = uadd_overflow_trap v21, v10, user18
;; @0022                               v51 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v24 = load.i64 notrap aligned v51+32
;; @0022                               v23 = uextend.i64 v22
;; @0022                               v25 = icmp ule v23, v24
;; @0022                               brif v25, block2, block3
;;
;;                                 block2:
;; @0022                               v32 = iconst.i32 -1476395008
;;                                     v74 = bor.i32 v10, v32  ; v32 = -1476395008
;; @0022                               v29 = load.i64 notrap aligned readonly can_move v51+24
;;                                     v95 = band.i32 v19, v73  ; v73 = -8
;;                                     v96 = uextend.i64 v95
;; @0022                               v31 = iadd v29, v96
;; @0022                               store notrap aligned v74, v31
;; @0022                               v35 = load.i64 notrap aligned readonly can_move v0+40
;; @0022                               v36 = load.i32 notrap aligned readonly can_move v35
;; @0022                               store notrap aligned v36, v31+4
;; @0022                               store.i32 notrap aligned v22, v15
;;                                     v54 = iconst.i64 8
;; @0022                               v37 = iadd v31, v54  ; v54 = 8
;; @0022                               store.i32 notrap aligned v3, v37
;;                                     v77 = iconst.i64 16
;;                                     v83 = iadd v31, v77  ; v77 = 16
;; @0022                               v43 = uextend.i64 v10
;; @0022                               v44 = iadd v31, v43
;; @0022                               jump block4(v83)
;;
;;                                 block4(v45: i64):
;; @0022                               v46 = icmp eq v45, v44
;; @0022                               brif v46, block6, block5
;;
;;                                 block5:
;; @0022                               store.i64 notrap aligned little v2, v45
;;                                     v97 = iconst.i64 8
;;                                     v98 = iadd.i64 v45, v97  ; v97 = 8
;; @0022                               jump block4(v98)
;;
;;                                 block6:
;; @0025                               jump block1
;;
;;                                 block3 cold:
;; @0022                               v27 = isub.i64 v23, v24
;; @0022                               v28 = call fn0(v0, v27)
;; @0022                               jump block2
;;
;;                                 block1:
;;                                     v99 = band.i32 v19, v73  ; v73 = -8
;; @0025                               return v99
;; }
