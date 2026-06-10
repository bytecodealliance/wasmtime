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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 32 "VMContext+0x20"
;;     region2 = 2147483648 "GcHeap"
;;     region3 = 40 "VMContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     fn0 = colocated u805306368:23 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0022                               v6 = uextend.i64 v3
;;                                     v83 = iconst.i64 3
;;                                     v84 = ishl v6, v83  ; v83 = 3
;; @0022                               v9 = iconst.i64 32
;; @0022                               v10 = ushr v84, v9  ; v9 = 32
;; @0022                               trapnz v10, user18
;; @0022                               v5 = iconst.i32 16
;;                                     v90 = iconst.i32 3
;;                                     v91 = ishl v3, v90  ; v90 = 3
;; @0022                               v12 = uadd_overflow_trap v5, v91, user18  ; v5 = 16
;; @0022                               v14 = iconst.i32 -67108864
;; @0022                               v15 = band v12, v14  ; v14 = -67108864
;; @0022                               trapnz v15, user18
;; @0022                               v16 = load.i64 notrap aligned readonly region1 v0+32
;; @0022                               v17 = load.i32 user2 region2 v16
;;                                     v94 = iconst.i32 7
;; @0022                               v20 = uadd_overflow_trap v17, v94, user18  ; v94 = 7
;;                                     v100 = iconst.i32 -8
;; @0022                               v22 = band v20, v100  ; v100 = -8
;; @0022                               v23 = uadd_overflow_trap v22, v12, user18
;; @0022                               v25 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0022                               v26 = load.i64 notrap aligned v25+40
;; @0022                               v24 = uextend.i64 v23
;; @0022                               v27 = icmp ule v24, v26
;; @0022                               brif v27, block2, block3
;;
;;                                 block2:
;; @0022                               v34 = iconst.i32 -1476395008
;;                                     v101 = bor.i32 v12, v34  ; v34 = -1476395008
;; @0022                               v31 = load.i64 notrap aligned readonly can_move v25+32
;;                                     v118 = band.i32 v20, v100  ; v100 = -8
;;                                     v119 = uextend.i64 v118
;; @0022                               v33 = iadd v31, v119
;; @0022                               store user2 region2 v101, v33
;; @0022                               v36 = load.i64 notrap aligned readonly can_move region3 v0+40
;; @0022                               v37 = load.i32 notrap aligned readonly can_move v36
;; @0022                               store user2 region2 v37, v33+4
;; @0022                               store.i32 user2 region2 v23, v16
;; @0022                               v7 = iconst.i64 8
;; @0022                               v39 = iadd v33, v7  ; v7 = 8
;; @0022                               store.i32 user2 region2 v3, v39
;; @0022                               trapz v118, user16
;; @0022                               v70 = load.i64 notrap aligned v25+40
;; @0022                               v58 = iconst.i64 16
;; @0022                               v59 = iadd v33, v58  ; v58 = 16
;; @0022                               v72 = uadd_overflow_trap v59, v84, user2
;; @0022                               v71 = iadd v31, v70
;; @0022                               v73 = icmp ugt v72, v71
;; @0022                               trapnz v73, user2
;;                                     v103 = iconst.i64 0
;; @0022                               v76 = icmp.i64 eq v6, v103  ; v103 = 0
;; @0022                               v74 = iadd v59, v84
;; @0022                               brif v76, block5, block4(v59)
;;
;;                                 block4(v77: i64):
;; @0022                               store.i64 user2 little region2 v2, v77
;;                                     v120 = iconst.i64 8
;;                                     v121 = iadd v77, v120  ; v120 = 8
;; @0022                               v80 = icmp eq v121, v74
;; @0022                               brif v80, block5, block4(v121)
;;
;;                                 block5:
;; @0025                               jump block1
;;
;;                                 block3 cold:
;; @0022                               v28 = isub.i64 v24, v26
;; @0022                               v29 = call fn0(v0, v28)
;; @0022                               jump block2
;;
;;                                 block1:
;;                                     v122 = band.i32 v20, v100  ; v100 = -8
;; @0025                               return v122
;; }
