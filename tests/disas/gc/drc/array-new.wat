;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $ty (array (mut i64)))

  (func (param i64 i32) (result (ref $ty))
    (array.new $ty (local.get 0) (local.get 1))
  )
)
;; function u0:0(i64 vmctx, i64, i64, i32) -> i32 tail {
;;     region0 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0022                               v6 = uextend.i64 v3
;;                                     v73 = iconst.i64 3
;;                                     v74 = ishl v6, v73  ; v73 = 3
;; @0022                               v9 = iconst.i64 32
;; @0022                               v10 = ushr v74, v9  ; v9 = 32
;; @0022                               trapnz v10, user18
;; @0022                               v5 = iconst.i32 32
;;                                     v80 = iconst.i32 3
;;                                     v81 = ishl v3, v80  ; v80 = 3
;; @0022                               v12 = uadd_overflow_trap v5, v81, user18  ; v5 = 32
;; @0022                               v14 = iconst.i32 -1476395008
;; @0022                               v16 = load.i64 notrap aligned readonly can_move v0+40
;; @0022                               v17 = load.i32 notrap aligned readonly can_move v16
;;                                     v78 = iconst.i32 8
;; @0022                               v19 = call fn0(v0, v14, v17, v12, v78)  ; v14 = -1476395008, v78 = 8
;; @0022                               v71 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v20 = load.i64 notrap aligned readonly can_move v71+32
;; @0022                               v21 = uextend.i64 v19
;; @0022                               v22 = iadd v20, v21
;; @0022                               v23 = iconst.i64 24
;; @0022                               v24 = iadd v22, v23  ; v23 = 24
;; @0022                               store user2 region0 v3, v24
;; @0022                               trapz v19, user16
;; @0022                               v52 = load.i64 notrap aligned v71+40
;; @0022                               v43 = iadd v22, v9  ; v9 = 32
;; @0022                               v54 = uadd_overflow_trap v43, v74, user2
;; @0022                               v53 = iadd v20, v52
;; @0022                               v55 = icmp ugt v54, v53
;; @0022                               trapnz v55, user2
;;                                     v84 = iconst.i64 0
;; @0022                               v58 = icmp eq v6, v84  ; v84 = 0
;; @0022                               v7 = iconst.i64 8
;; @0022                               v56 = iadd v43, v74
;; @0022                               brif v58, block3, block2(v43)
;;
;;                                 block2(v59: i64):
;; @0022                               store.i64 user2 little region0 v2, v59
;;                                     v99 = iconst.i64 8
;;                                     v100 = iadd v59, v99  ; v99 = 8
;; @0022                               v62 = icmp eq v100, v56
;; @0022                               brif v62, block3, block2(v100)
;;
;;                                 block3:
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v19
;; }
