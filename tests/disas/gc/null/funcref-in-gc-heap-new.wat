;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut funcref))))

  (func (param funcref) (result (ref $ty))
    (struct.new $ty (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i64) -> i32 tail {
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
;;     sig1 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:23 sig0
;;     fn1 = colocated u805306368:25 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;; @0020                               v7 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @0020                               v8 = load.i32 notrap aligned region3 v7
;;                                     v39 = iconst.i32 7
;; @0020                               v11 = uadd_overflow_trap v8, v39, user18  ; v39 = 7
;;                                     v45 = iconst.i32 -8
;; @0020                               v13 = band v11, v45  ; v45 = -8
;; @0020                               v3 = iconst.i32 16
;; @0020                               v14 = uadd_overflow_trap v13, v3, user18  ; v3 = 16
;; @0020                               v16 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0020                               v17 = load.i64 notrap aligned region4 v16+40
;; @0020                               v15 = uextend.i64 v14
;; @0020                               v18 = icmp ule v15, v17
;; @0020                               brif v18, block2, block3
;;
;;                                 block2:
;;                                     v46 = iconst.i32 -1342177264
;; @0020                               v22 = load.i64 notrap aligned readonly can_move region5 v16+32
;;                                     v52 = band.i32 v11, v45  ; v45 = -8
;;                                     v53 = uextend.i64 v52
;; @0020                               v24 = iadd v22, v53
;; @0020                               store user2 region7 v46, v24  ; v46 = -1342177264
;; @0020                               v27 = load.i64 notrap aligned readonly can_move region6 v0+40
;; @0020                               v28 = load.i32 notrap aligned readonly can_move v27
;; @0020                               store user2 region7 v28, v24+4
;; @0020                               store.i32 notrap aligned region3 v14, v7
;; @0020                               v31 = call fn1(v0, v2)
;; @0020                               v32 = ireduce.i32 v31
;; @0020                               v29 = iconst.i64 8
;; @0020                               v30 = iadd v24, v29  ; v29 = 8
;; @0020                               store user2 little region7 v32, v30
;; @0023                               jump block1
;;
;;                                 block3 cold:
;; @0020                               v19 = isub.i64 v15, v17
;; @0020                               v20 = call fn0(v0, v19)
;; @0020                               jump block2
;;
;;                                 block1:
;;                                     v54 = band.i32 v11, v45  ; v45 = -8
;; @0023                               return v54
;; }
