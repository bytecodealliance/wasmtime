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
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 32 "VMContext+0x20"
;;     region3 = 872415232 "VMCopyingHeapData+0x0"
;;     region4 = 872415236 "VMCopyingHeapData+0x4"
;;     region5 = 40 "VMContext+0x28"
;;     region6 = 1677721600 "TypeIdsArray+0x0"
;;     region7 = 67108896 "VMStoreContext+0x20"
;;     region8 = 536870912 "GcHeap"
;;     region9 = 1543503872 "Stack(ss0)"
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
;;                                     v43 = iconst.i64 32
;; @0020                               v13 = iadd v12, v43  ; v43 = 32
;; @0020                               v14 = uextend.i64 v6
;; @0020                               v15 = icmp ule v13, v14
;; @0020                               brif v15, block2, block3
;;
;;                                 block2:
;;                                     v59 = iconst.i32 32
;;                                     v57 = iadd.i32 v5, v59  ; v59 = 32
;; @0020                               store notrap aligned region3 v57, v4
;;                                     v60 = iconst.i32 -1342177278
;;                                     v61 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v62 = load.i64 notrap aligned readonly can_move region7 v61+32
;; @0020                               v29 = iadd v62, v12
;; @0020                               store user2 region8 v60, v29  ; v60 = -1342177278
;;                                     v63 = load.i64 notrap aligned readonly can_move region5 v0+40
;;                                     v64 = load.i32 notrap aligned readonly can_move region6 v63
;; @0020                               store user2 region8 v64, v29+4
;; @0020                               store user2 region8 v59, v29+8  ; v59 = 32
;; @0020                               jump block4(v5, v29)
;;
;;                                 block3 cold:
;; @0020                               v16 = iconst.i32 -1342177278
;; @0020                               v17 = load.i64 notrap aligned readonly can_move region5 v0+40
;; @0020                               v18 = load.i32 notrap aligned readonly can_move region6 v17
;; @0020                               v3 = iconst.i32 32
;; @0020                               v19 = iconst.i32 16
;; @0020                               v20 = call fn0(v0, v16, v18, v3, v19)  ; v16 = -1342177278, v3 = 32, v19 = 16
;; @0020                               v21 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0020                               v22 = load.i64 notrap aligned readonly can_move region7 v21+32
;; @0020                               v23 = uextend.i64 v20
;; @0020                               v24 = iadd v22, v23
;; @0020                               jump block4(v20, v24)
;;
;;                                 block4(v34: i32, v35: i64):
;;                                     v42 = stack_addr.i64 ss0
;;                                     store notrap aligned region9 v34, v42
;; @0020                               v38 = call fn1(v0, v2), stack_map=[i32 @ ss0+0]
;; @0020                               v39 = ireduce.i32 v38
;; @0020                               v36 = iconst.i64 16
;; @0020                               v37 = iadd v35, v36  ; v36 = 16
;; @0020                               store user2 little region8 v39, v37
;; @0023                               jump block1
;;
;;                                 block1:
;;                                     v41 = load.i32 notrap aligned region9 v42
;; @0023                               return v41
;; }
