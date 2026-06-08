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
;;     region0 = 32 "VMContext+0x20"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:24 sig0
;;     fn1 = colocated u805306368:25 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;; @0020                               v5 = load.i64 notrap aligned readonly can_move v0+32
;; @0020                               v6 = load.i32 notrap aligned v5
;; @0020                               v7 = load.i32 notrap aligned v5+4
;; @0020                               v13 = uextend.i64 v6
;;                                     v45 = iconst.i64 32
;; @0020                               v14 = iadd v13, v45  ; v45 = 32
;; @0020                               v15 = uextend.i64 v7
;; @0020                               v16 = icmp ule v14, v15
;; @0020                               brif v16, block2, block3
;;
;;                                 block2:
;;                                     v61 = iconst.i32 32
;;                                     v59 = iadd.i32 v6, v61  ; v61 = 32
;; @0020                               store notrap aligned region0 v59, v5
;;                                     v62 = iconst.i32 -1342177278
;;                                     v63 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v64 = load.i64 notrap aligned readonly can_move v63+32
;; @0020                               v28 = iadd v64, v13
;; @0020                               store notrap aligned v62, v28  ; v62 = -1342177278
;;                                     v65 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v66 = load.i32 notrap aligned readonly can_move v65
;; @0020                               store notrap aligned v66, v28+4
;;                                     v67 = iconst.i64 32
;; @0020                               istore32 notrap aligned v67, v28+8  ; v67 = 32
;; @0020                               jump block4(v6, v28)
;;
;;                                 block3 cold:
;; @0020                               v17 = iconst.i32 -1342177278
;; @0020                               v18 = load.i64 notrap aligned readonly can_move v0+40
;; @0020                               v19 = load.i32 notrap aligned readonly can_move v18
;; @0020                               v4 = iconst.i32 32
;; @0020                               v20 = iconst.i32 16
;; @0020                               v21 = call fn0(v0, v17, v19, v4, v20)  ; v17 = -1342177278, v4 = 32, v20 = 16
;; @0020                               v41 = load.i64 notrap aligned readonly can_move v0+8
;; @0020                               v22 = load.i64 notrap aligned readonly can_move v41+32
;; @0020                               v23 = uextend.i64 v21
;; @0020                               v24 = iadd v22, v23
;; @0020                               jump block4(v21, v24)
;;
;;                                 block4(v32: i32, v33: i64):
;;                                     v40 = stack_addr.i64 ss0
;;                                     store notrap v32, v40
;; @0020                               v36 = call fn1(v0, v2), stack_map=[i32 @ ss0+0]
;; @0020                               v37 = ireduce.i32 v36
;; @0020                               v34 = iconst.i64 16
;; @0020                               v35 = iadd v33, v34  ; v34 = 16
;; @0020                               store user2 little region1 v37, v35
;;                                     v39 = load.i32 notrap v40
;; @0023                               jump block1
;;
;;                                 block1:
;; @0023                               return v39
;; }
