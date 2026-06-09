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
;;                                     v68 = iconst.i64 3
;;                                     v69 = ishl v6, v68  ; v68 = 3
;; @0022                               v9 = iconst.i64 32
;; @0022                               v10 = ushr v69, v9  ; v9 = 32
;; @0022                               trapnz v10, user18
;; @0022                               v5 = iconst.i32 32
;;                                     v75 = iconst.i32 3
;;                                     v76 = ishl v3, v75  ; v75 = 3
;; @0022                               v12 = uadd_overflow_trap v5, v76, user18  ; v5 = 32
;; @0022                               v13 = iconst.i32 -1476395008
;; @0022                               v14 = load.i64 notrap aligned readonly can_move v0+40
;; @0022                               v15 = load.i32 notrap aligned readonly can_move v14
;;                                     v73 = iconst.i32 8
;; @0022                               v17 = call fn0(v0, v13, v15, v12, v73)  ; v13 = -1476395008, v73 = 8
;; @0022                               v18 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v19 = load.i64 notrap aligned readonly can_move v18+32
;; @0022                               v20 = uextend.i64 v17
;; @0022                               v21 = iadd v19, v20
;; @0022                               v22 = iconst.i64 24
;; @0022                               v23 = iadd v21, v22  ; v22 = 24
;; @0022                               store user2 region0 v3, v23
;; @0022                               trapz v17, user16
;; @0022                               v53 = load.i64 notrap aligned v18+40
;; @0022                               v43 = iadd v21, v9  ; v9 = 32
;; @0022                               v55 = uadd_overflow_trap v43, v69, user2
;; @0022                               v54 = iadd v19, v53
;; @0022                               v56 = icmp ugt v55, v54
;; @0022                               trapnz v56, user2
;;                                     v79 = iconst.i64 0
;; @0022                               v59 = icmp eq v6, v79  ; v79 = 0
;; @0022                               v7 = iconst.i64 8
;; @0022                               v57 = iadd v43, v69
;; @0022                               brif v59, block3, block2(v43)
;;
;;                                 block2(v60: i64):
;; @0022                               store.i64 user2 little region0 v2, v60
;;                                     v94 = iconst.i64 8
;;                                     v95 = iadd v60, v94  ; v94 = 8
;; @0022                               v63 = icmp eq v95, v57
;; @0022                               brif v63, block3, block2(v95)
;;
;;                                 block3:
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v17
;; }
