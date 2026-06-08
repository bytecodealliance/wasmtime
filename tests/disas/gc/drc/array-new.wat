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
;;                                     v71 = iconst.i64 3
;;                                     v72 = ishl v6, v71  ; v71 = 3
;; @0022                               v9 = iconst.i64 32
;; @0022                               v10 = ushr v72, v9  ; v9 = 32
;; @0022                               trapnz v10, user18
;; @0022                               v5 = iconst.i32 32
;;                                     v78 = iconst.i32 3
;;                                     v79 = ishl v3, v78  ; v78 = 3
;; @0022                               v12 = uadd_overflow_trap v5, v79, user18  ; v5 = 32
;; @0022                               v13 = iconst.i32 -1476395008
;; @0022                               v14 = load.i64 notrap aligned readonly can_move v0+40
;; @0022                               v15 = load.i32 notrap aligned readonly can_move v14
;;                                     v76 = iconst.i32 8
;; @0022                               v17 = call fn0(v0, v13, v15, v12, v76)  ; v13 = -1476395008, v76 = 8
;; @0022                               v69 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v18 = load.i64 notrap aligned readonly can_move v69+32
;; @0022                               v19 = uextend.i64 v17
;; @0022                               v20 = iadd v18, v19
;; @0022                               v21 = iconst.i64 24
;; @0022                               v22 = iadd v20, v21  ; v21 = 24
;; @0022                               store user2 region0 v3, v22
;; @0022                               trapz v17, user16
;; @0022                               v50 = load.i64 notrap aligned v69+40
;; @0022                               v41 = iadd v20, v9  ; v9 = 32
;; @0022                               v52 = uadd_overflow_trap v41, v72, user2
;; @0022                               v51 = iadd v18, v50
;; @0022                               v53 = icmp ugt v52, v51
;; @0022                               trapnz v53, user2
;;                                     v82 = iconst.i64 0
;; @0022                               v56 = icmp eq v6, v82  ; v82 = 0
;; @0022                               v7 = iconst.i64 8
;; @0022                               v54 = iadd v41, v72
;; @0022                               brif v56, block3, block2(v41)
;;
;;                                 block2(v57: i64):
;; @0022                               store.i64 user2 little region0 v2, v57
;;                                     v97 = iconst.i64 8
;;                                     v98 = iadd v57, v97  ; v97 = 8
;; @0022                               v60 = icmp eq v98, v54
;; @0022                               brif v60, block3, block2(v98)
;;
;;                                 block3:
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v17
;; }
