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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 40 "VMContext+0x28"
;;     region2 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0022                               v6 = uextend.i64 v3
;;                                     v67 = iconst.i64 3
;;                                     v68 = ishl v6, v67  ; v67 = 3
;; @0022                               v9 = iconst.i64 32
;; @0022                               v10 = ushr v68, v9  ; v9 = 32
;; @0022                               trapnz v10, user18
;; @0022                               v5 = iconst.i32 32
;;                                     v74 = iconst.i32 3
;;                                     v75 = ishl v3, v74  ; v74 = 3
;; @0022                               v12 = uadd_overflow_trap v5, v75, user18  ; v5 = 32
;; @0022                               v13 = iconst.i32 -1476395008
;; @0022                               v14 = load.i64 notrap aligned readonly can_move region1 v0+40
;; @0022                               v15 = load.i32 notrap aligned readonly can_move v14
;;                                     v72 = iconst.i32 8
;; @0022                               v17 = call fn0(v0, v13, v15, v12, v72)  ; v13 = -1476395008, v72 = 8
;; @0022                               v18 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0022                               v19 = load.i64 notrap aligned readonly can_move v18+32
;; @0022                               v20 = uextend.i64 v17
;; @0022                               v21 = iadd v19, v20
;; @0022                               v22 = iconst.i64 24
;; @0022                               v23 = iadd v21, v22  ; v22 = 24
;; @0022                               store user2 region2 v3, v23
;; @0022                               trapz v17, user16
;; @0022                               v54 = load.i64 notrap aligned v18+40
;; @0022                               v43 = iadd v21, v9  ; v9 = 32
;; @0022                               v56 = uadd_overflow_trap v43, v68, user2
;; @0022                               v55 = iadd v19, v54
;; @0022                               v57 = icmp ugt v56, v55
;; @0022                               trapnz v57, user2
;;                                     v78 = iconst.i64 0
;; @0022                               v60 = icmp eq v6, v78  ; v78 = 0
;; @0022                               v7 = iconst.i64 8
;; @0022                               v58 = iadd v43, v68
;; @0022                               brif v60, block3, block2(v43)
;;
;;                                 block2(v61: i64):
;; @0022                               store.i64 user2 little region2 v2, v61
;;                                     v93 = iconst.i64 8
;;                                     v94 = iadd v61, v93  ; v93 = 8
;; @0022                               v64 = icmp eq v94, v58
;; @0022                               brif v64, block3, block2(v94)
;;
;;                                 block3:
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v17
;; }
