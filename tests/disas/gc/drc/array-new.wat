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
;; @0022                               v5 = uextend.i64 v3
;;                                     v65 = iconst.i64 3
;;                                     v66 = ishl v5, v65  ; v65 = 3
;; @0022                               v8 = iconst.i64 32
;; @0022                               v9 = ushr v66, v8  ; v8 = 32
;; @0022                               trapnz v9, user18
;; @0022                               v4 = iconst.i32 32
;;                                     v72 = iconst.i32 3
;;                                     v73 = ishl v3, v72  ; v72 = 3
;; @0022                               v11 = uadd_overflow_trap v4, v73, user18  ; v4 = 32
;; @0022                               v12 = iconst.i32 -1476395008
;; @0022                               v13 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @0022                               v14 = load.i32 notrap aligned readonly can_move v13
;;                                     v70 = iconst.i32 8
;; @0022                               v16 = call fn0(v0, v12, v14, v11, v70)  ; v12 = -1476395008, v70 = 8
;; @0022                               v17 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0022                               v18 = load.i64 notrap aligned readonly can_move region3 v17+32
;; @0022                               v19 = uextend.i64 v16
;; @0022                               v20 = iadd v18, v19
;; @0022                               v21 = iconst.i64 24
;; @0022                               v22 = iadd v20, v21  ; v21 = 24
;; @0022                               store user2 region4 v3, v22
;; @0022                               trapz v16, user16
;; @0022                               v54 = load.i64 notrap aligned region5 v17+40
;; @0022                               v43 = iadd v20, v8  ; v8 = 32
;; @0022                               v56 = uadd_overflow_trap v43, v66, user2
;; @0022                               v55 = iadd v18, v54
;; @0022                               v57 = icmp ugt v56, v55
;; @0022                               trapnz v57, user2
;;                                     v76 = iconst.i64 0
;; @0022                               v60 = icmp eq v5, v76  ; v76 = 0
;; @0022                               v6 = iconst.i64 8
;; @0022                               v58 = iadd v43, v66
;; @0022                               brif v60, block3, block2(v43)
;;
;;                                 block2(v61: i64):
;; @0022                               store.i64 user2 little region4 v2, v61
;;                                     v91 = iconst.i64 8
;;                                     v92 = iadd v61, v91  ; v91 = 8
;; @0022                               v64 = icmp eq v92, v58
;; @0022                               brif v64, block3, block2(v92)
;;
;;                                 block3:
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v16
;; }
