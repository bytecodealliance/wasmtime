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
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 40 "VMContext+0x28"
;;     region3 = 268435488 "VMStoreContext+0x20"
;;     region4 = 2147483648 "GcHeap"
;;     region5 = 268435496 "VMStoreContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0022                               v6 = uextend.i64 v3
;;                                     v66 = iconst.i64 3
;;                                     v67 = ishl v6, v66  ; v66 = 3
;; @0022                               v9 = iconst.i64 32
;; @0022                               v10 = ushr v67, v9  ; v9 = 32
;; @0022                               trapnz v10, user18
;; @0022                               v5 = iconst.i32 32
;;                                     v73 = iconst.i32 3
;;                                     v74 = ishl v3, v73  ; v73 = 3
;; @0022                               v12 = uadd_overflow_trap v5, v74, user18  ; v5 = 32
;; @0022                               v13 = iconst.i32 -1476395008
;; @0022                               v14 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @0022                               v15 = load.i32 notrap aligned readonly can_move v14
;;                                     v71 = iconst.i32 8
;; @0022                               v17 = call fn0(v0, v13, v15, v12, v71)  ; v13 = -1476395008, v71 = 8
;; @0022                               v18 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0022                               v19 = load.i64 notrap aligned readonly can_move region3 v18+32
;; @0022                               v20 = uextend.i64 v17
;; @0022                               v21 = iadd v19, v20
;; @0022                               v22 = iconst.i64 24
;; @0022                               v23 = iadd v21, v22  ; v22 = 24
;; @0022                               store user2 region4 v3, v23
;; @0022                               trapz v17, user16
;; @0022                               v55 = load.i64 notrap aligned region5 v18+40
;; @0022                               v44 = iadd v21, v9  ; v9 = 32
;; @0022                               v57 = uadd_overflow_trap v44, v67, user2
;; @0022                               v56 = iadd v19, v55
;; @0022                               v58 = icmp ugt v57, v56
;; @0022                               trapnz v58, user2
;;                                     v77 = iconst.i64 0
;; @0022                               v61 = icmp eq v6, v77  ; v77 = 0
;; @0022                               v7 = iconst.i64 8
;; @0022                               v59 = iadd v44, v67
;; @0022                               brif v61, block3, block2(v44)
;;
;;                                 block2(v62: i64):
;; @0022                               store.i64 user2 little region4 v2, v62
;;                                     v92 = iconst.i64 8
;;                                     v93 = iadd v62, v92  ; v92 = 8
;; @0022                               v65 = icmp eq v93, v59
;; @0022                               brif v65, block3, block2(v93)
;;
;;                                 block3:
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v17
;; }
