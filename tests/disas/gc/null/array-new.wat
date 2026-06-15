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
;;     region3 = 3758096384 "VMNullHeapData+0x0"
;;     region4 = 268435496 "VMStoreContext+0x28"
;;     region5 = 268435488 "VMStoreContext+0x20"
;;     region6 = 40 "VMContext+0x28"
;;     region7 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     fn0 = colocated u805306368:23 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0022                               v5 = uextend.i64 v3
;;                                     v81 = iconst.i64 3
;;                                     v82 = ishl v5, v81  ; v81 = 3
;; @0022                               v8 = iconst.i64 32
;; @0022                               v9 = ushr v82, v8  ; v8 = 32
;; @0022                               trapnz v9, user18
;; @0022                               v4 = iconst.i32 16
;;                                     v88 = iconst.i32 3
;;                                     v89 = ishl v3, v88  ; v88 = 3
;; @0022                               v11 = uadd_overflow_trap v4, v89, user18  ; v4 = 16
;; @0022                               v13 = iconst.i32 -67108864
;; @0022                               v14 = band v11, v13  ; v13 = -67108864
;; @0022                               trapnz v14, user18
;; @0022                               v15 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @0022                               v16 = load.i32 notrap aligned region3 v15
;;                                     v92 = iconst.i32 7
;; @0022                               v19 = uadd_overflow_trap v16, v92, user18  ; v92 = 7
;;                                     v98 = iconst.i32 -8
;; @0022                               v21 = band v19, v98  ; v98 = -8
;; @0022                               v22 = uadd_overflow_trap v21, v11, user18
;; @0022                               v24 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0022                               v25 = load.i64 notrap aligned region4 v24+40
;; @0022                               v23 = uextend.i64 v22
;; @0022                               v26 = icmp ule v23, v25
;; @0022                               brif v26, block2, block3
;;
;;                                 block2:
;; @0022                               v33 = iconst.i32 -1476395008
;;                                     v99 = bor.i32 v11, v33  ; v33 = -1476395008
;; @0022                               v30 = load.i64 notrap aligned readonly can_move region5 v24+32
;;                                     v116 = band.i32 v19, v98  ; v98 = -8
;;                                     v117 = uextend.i64 v116
;; @0022                               v32 = iadd v30, v117
;; @0022                               store user2 region7 v99, v32
;; @0022                               v35 = load.i64 notrap aligned readonly can_move region6 v0+40
;; @0022                               v36 = load.i32 notrap aligned readonly can_move v35
;; @0022                               store user2 region7 v36, v32+4
;; @0022                               store.i32 notrap aligned region3 v22, v15
;; @0022                               v6 = iconst.i64 8
;; @0022                               v38 = iadd v32, v6  ; v6 = 8
;; @0022                               store.i32 user2 region7 v3, v38
;; @0022                               trapz v116, user16
;; @0022                               v70 = load.i64 notrap aligned region4 v24+40
;; @0022                               v58 = iconst.i64 16
;; @0022                               v59 = iadd v32, v58  ; v58 = 16
;; @0022                               v72 = uadd_overflow_trap v59, v82, user2
;; @0022                               v71 = iadd v30, v70
;; @0022                               v73 = icmp ugt v72, v71
;; @0022                               trapnz v73, user2
;;                                     v101 = iconst.i64 0
;; @0022                               v76 = icmp.i64 eq v5, v101  ; v101 = 0
;; @0022                               v74 = iadd v59, v82
;; @0022                               brif v76, block5, block4(v59)
;;
;;                                 block4(v77: i64):
;; @0022                               store.i64 user2 little region7 v2, v77
;;                                     v118 = iconst.i64 8
;;                                     v119 = iadd v77, v118  ; v118 = 8
;; @0022                               v80 = icmp eq v119, v74
;; @0022                               brif v80, block5, block4(v119)
;;
;;                                 block5:
;; @0025                               jump block1
;;
;;                                 block3 cold:
;; @0022                               v27 = isub.i64 v23, v25
;; @0022                               v28 = call fn0(v0, v27)
;; @0022                               jump block2
;;
;;                                 block1:
;;                                     v120 = band.i32 v19, v98  ; v98 = -8
;; @0025                               return v120
;; }
