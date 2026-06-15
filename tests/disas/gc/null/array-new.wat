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
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 32 "VMContext+0x20"
;;     region3 = 2147483648 "GcHeap"
;;     region4 = 268435496 "VMStoreContext+0x28"
;;     region5 = 268435488 "VMStoreContext+0x20"
;;     region6 = 40 "VMContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     fn0 = colocated u805306368:23 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0022                               v6 = uextend.i64 v3
;;                                     v82 = iconst.i64 3
;;                                     v83 = ishl v6, v82  ; v82 = 3
;; @0022                               v9 = iconst.i64 32
;; @0022                               v10 = ushr v83, v9  ; v9 = 32
;; @0022                               trapnz v10, user18
;; @0022                               v5 = iconst.i32 16
;;                                     v89 = iconst.i32 3
;;                                     v90 = ishl v3, v89  ; v89 = 3
;; @0022                               v12 = uadd_overflow_trap v5, v90, user18  ; v5 = 16
;; @0022                               v14 = iconst.i32 -67108864
;; @0022                               v15 = band v12, v14  ; v14 = -67108864
;; @0022                               trapnz v15, user18
;; @0022                               v16 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @0022                               v17 = load.i32 user2 region3 v16
;;                                     v93 = iconst.i32 7
;; @0022                               v20 = uadd_overflow_trap v17, v93, user18  ; v93 = 7
;;                                     v99 = iconst.i32 -8
;; @0022                               v22 = band v20, v99  ; v99 = -8
;; @0022                               v23 = uadd_overflow_trap v22, v12, user18
;; @0022                               v25 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0022                               v26 = load.i64 notrap aligned region4 v25+40
;; @0022                               v24 = uextend.i64 v23
;; @0022                               v27 = icmp ule v24, v26
;; @0022                               brif v27, block2, block3
;;
;;                                 block2:
;; @0022                               v34 = iconst.i32 -1476395008
;;                                     v100 = bor.i32 v12, v34  ; v34 = -1476395008
;; @0022                               v31 = load.i64 notrap aligned readonly can_move region5 v25+32
;;                                     v117 = band.i32 v20, v99  ; v99 = -8
;;                                     v118 = uextend.i64 v117
;; @0022                               v33 = iadd v31, v118
;; @0022                               store user2 region3 v100, v33
;; @0022                               v36 = load.i64 notrap aligned readonly can_move region6 v0+40
;; @0022                               v37 = load.i32 notrap aligned readonly can_move v36
;; @0022                               store user2 region3 v37, v33+4
;; @0022                               store.i32 user2 region3 v23, v16
;; @0022                               v7 = iconst.i64 8
;; @0022                               v39 = iadd v33, v7  ; v7 = 8
;; @0022                               store.i32 user2 region3 v3, v39
;; @0022                               trapz v117, user16
;; @0022                               v71 = load.i64 notrap aligned region4 v25+40
;; @0022                               v59 = iconst.i64 16
;; @0022                               v60 = iadd v33, v59  ; v59 = 16
;; @0022                               v73 = uadd_overflow_trap v60, v83, user2
;; @0022                               v72 = iadd v31, v71
;; @0022                               v74 = icmp ugt v73, v72
;; @0022                               trapnz v74, user2
;;                                     v102 = iconst.i64 0
;; @0022                               v77 = icmp.i64 eq v6, v102  ; v102 = 0
;; @0022                               v75 = iadd v60, v83
;; @0022                               brif v77, block5, block4(v60)
;;
;;                                 block4(v78: i64):
;; @0022                               store.i64 user2 little region3 v2, v78
;;                                     v119 = iconst.i64 8
;;                                     v120 = iadd v78, v119  ; v119 = 8
;; @0022                               v81 = icmp eq v120, v75
;; @0022                               brif v81, block5, block4(v120)
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
;;                                     v121 = band.i32 v20, v99  ; v99 = -8
;; @0025                               return v121
;; }
