;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
;;! test = "optimize"
(module
  (type $ty (struct (field (mut funcref))))

  (func (param funcref) (result (ref $ty))
    (struct.new $ty (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i64) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 32 "VMContext+0x20"
;;     region3 = 3489660928 "VMCopyingHeapData+0x0"
;;     region4 = 3489660932 "VMCopyingHeapData+0x4"
;;     region5 = 40 "VMContext+0x28"
;;     region6 = 268435488 "VMStoreContext+0x20"
;;     region7 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:24 sig0
;;     fn1 = colocated u805306368:25 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;; @0020                               v4 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @0020                               v5 = load.i32 notrap aligned region3 v4
;; @0020                               v6 = load.i32 notrap aligned region4 v4+4
;; @0020                               v12 = uextend.i64 v5
;;                                     v42 = iconst.i64 32
;; @0020                               v13 = iadd v12, v42  ; v42 = 32
;; @0020                               v14 = uextend.i64 v6
;; @0020                               v15 = icmp ule v13, v14
;; @0020                               brif v15, block2, block3
;;
;;                                 block2:
;;                                     v58 = iconst.i32 32
;;                                     v56 = iadd.i32 v5, v58  ; v58 = 32
;; @0020                               store notrap aligned region3 v56, v4
;;                                     v59 = iconst.i32 -1342177278
;;                                     v60 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v61 = load.i64 notrap aligned readonly can_move region6 v60+32
;; @0020                               v29 = iadd v61, v12
;; @0020                               store user2 region7 v59, v29  ; v59 = -1342177278
;;                                     v62 = load.i64 notrap aligned readonly can_move region5 v0+40
;;                                     v63 = load.i32 notrap aligned readonly can_move v62
;; @0020                               store user2 region7 v63, v29+4
;;                                     v64 = iconst.i64 32
;; @0020                               istore32 user2 region7 v64, v29+8  ; v64 = 32
;; @0020                               jump block4(v5, v29)
;;
;;                                 block3 cold:
;; @0020                               v16 = iconst.i32 -1342177278
;; @0020                               v17 = load.i64 notrap aligned readonly can_move region5 v0+40
;; @0020                               v18 = load.i32 notrap aligned readonly can_move v17
;; @0020                               v3 = iconst.i32 32
;; @0020                               v19 = iconst.i32 16
;; @0020                               v20 = call fn0(v0, v16, v18, v3, v19)  ; v16 = -1342177278, v3 = 32, v19 = 16
;; @0020                               v21 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0020                               v22 = load.i64 notrap aligned readonly can_move region6 v21+32
;; @0020                               v23 = uextend.i64 v20
;; @0020                               v24 = iadd v22, v23
;; @0020                               jump block4(v20, v24)
;;
;;                                 block4(v33: i32, v34: i64):
;;                                     v41 = stack_addr.i64 ss0
;;                                     store notrap v33, v41
;; @0020                               v37 = call fn1(v0, v2), stack_map=[i32 @ ss0+0]
;; @0020                               v38 = ireduce.i32 v37
;; @0020                               v35 = iconst.i64 16
;; @0020                               v36 = iadd v34, v35  ; v35 = 16
;; @0020                               store user2 little region7 v38, v36
;; @0023                               jump block1
;;
;;                                 block1:
;;                                     v40 = load.i32 notrap v41
;; @0023                               return v40
;; }
