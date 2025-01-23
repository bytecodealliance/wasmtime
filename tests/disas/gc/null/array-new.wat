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
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0022                               v6 = uextend.i64 v3
;;                                     v50 = iconst.i64 3
;;                                     v51 = ishl v6, v50  ; v50 = 3
;;                                     v48 = iconst.i64 32
;; @0022                               v8 = ushr v51, v48  ; v48 = 32
;; @0022                               trapnz v8, user18
;; @0022                               v5 = iconst.i32 16
;;                                     v57 = iconst.i32 3
;;                                     v58 = ishl v3, v57  ; v57 = 3
;; @0022                               v10 = uadd_overflow_trap v5, v58, user18  ; v5 = 16
;; @0022                               v12 = iconst.i32 -134217728
;; @0022                               v13 = band v10, v12  ; v12 = -134217728
;; @0022                               trapnz v13, user18
;; @0022                               v15 = load.i64 notrap aligned readonly v0+56
;; @0022                               v16 = load.i32 notrap aligned v15
;;                                     v61 = iconst.i32 7
;; @0022                               v19 = uadd_overflow_trap v16, v61, user18  ; v61 = 7
;;                                     v68 = iconst.i32 -8
;; @0022                               v21 = band v19, v68  ; v68 = -8
;; @0022                               v22 = uadd_overflow_trap v21, v10, user18
;; @0022                               v23 = uextend.i64 v22
;; @0022                               v27 = load.i64 notrap aligned readonly v0+48
;; @0022                               v28 = icmp ule v23, v27
;; @0022                               trapz v28, user18
;; @0022                               v31 = iconst.i32 -1476395008
;;                                     v69 = bor v10, v31  ; v31 = -1476395008
;; @0022                               v25 = load.i64 notrap aligned readonly v0+40
;; @0022                               v29 = uextend.i64 v21
;; @0022                               v30 = iadd v25, v29
;; @0022                               store notrap aligned v69, v30
;; @0022                               v34 = load.i64 notrap aligned readonly v0+64
;; @0022                               v35 = load.i32 notrap aligned readonly v34
;; @0022                               store notrap aligned v35, v30+4
;; @0022                               store notrap aligned v22, v15
;;                                     v47 = iconst.i64 8
;; @0022                               v36 = iadd v30, v47  ; v47 = 8
;; @0022                               store notrap aligned v3, v36
;;                                     v72 = iconst.i64 16
;;                                     v78 = iadd v30, v72  ; v72 = 16
;; @0022                               v42 = uextend.i64 v10
;; @0022                               v43 = iadd v30, v42
;; @0022                               jump block2(v78)
;;
;;                                 block2(v44: i64):
;; @0022                               v45 = icmp eq v44, v43
;; @0022                               brif v45, block4, block3
;;
;;                                 block3:
;; @0022                               store.i64 notrap aligned little v2, v44
;;                                     v90 = iconst.i64 8
;;                                     v91 = iadd.i64 v44, v90  ; v90 = 8
;; @0022                               jump block2(v91)
;;
;;                                 block4:
;; @0025                               jump block1
;;
;;                                 block1:
;;                                     v92 = band.i32 v19, v68  ; v68 = -8
;; @0025                               return v92
;; }
