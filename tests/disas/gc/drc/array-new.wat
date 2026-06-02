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
;;                                     v72 = iconst.i64 32
;; @0022                               v9 = ushr v74, v72  ; v72 = 32
;; @0022                               trapnz v9, user18
;; @0022                               v5 = iconst.i32 32
;;                                     v80 = iconst.i32 3
;;                                     v81 = ishl v3, v80  ; v80 = 3
;; @0022                               v11 = uadd_overflow_trap v5, v81, user18  ; v5 = 32
;; @0022                               v13 = iconst.i32 -1476395008
;; @0022                               v15 = load.i64 notrap aligned readonly can_move v0+40
;; @0022                               v16 = load.i32 notrap aligned readonly can_move v15
;;                                     v78 = iconst.i32 8
;; @0022                               v18 = call fn0(v0, v13, v16, v11, v78)  ; v13 = -1476395008, v78 = 8
;; @0022                               v70 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v19 = load.i64 notrap aligned readonly can_move v70+32
;; @0022                               v20 = uextend.i64 v18
;; @0022                               v21 = iadd v19, v20
;; @0022                               v22 = iconst.i64 24
;; @0022                               v23 = iadd v21, v22  ; v22 = 24
;; @0022                               store user2 region0 v3, v23
;; @0022                               trapz v18, user16
;; @0022                               v51 = load.i64 notrap aligned v70+40
;; @0022                               v42 = iadd v21, v72  ; v72 = 32
;; @0022                               v53 = uadd_overflow_trap v42, v74, user2
;; @0022                               v52 = iadd v19, v51
;; @0022                               v54 = icmp ugt v53, v52
;; @0022                               trapnz v54, user2
;;                                     v84 = iconst.i64 0
;; @0022                               v57 = icmp eq v6, v84  ; v84 = 0
;; @0022                               v7 = iconst.i64 8
;; @0022                               v55 = iadd v42, v74
;; @0022                               brif v57, block3, block2(v42)
;;
;;                                 block2(v58: i64):
;; @0022                               store.i64 user2 little region0 v2, v58
;;                                     v99 = iconst.i64 8
;;                                     v100 = iadd v58, v99  ; v99 = 8
;; @0022                               v61 = icmp eq v100, v55
;; @0022                               brif v61, block3, block2(v100)
;;
;;                                 block3:
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v18
;; }
