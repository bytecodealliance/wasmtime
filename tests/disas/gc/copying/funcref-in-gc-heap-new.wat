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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:27 sig0
;;     fn1 = colocated u805306368:28 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;; @0020                               v6 = load.i64 notrap aligned readonly can_move v0+32
;; @0020                               v7 = load.i32 notrap aligned v6
;; @0020                               v8 = load.i32 notrap aligned v6+4
;; @0020                               v14 = uextend.i64 v7
;;                                     v50 = iconst.i64 32
;; @0020                               v15 = iadd v14, v50  ; v50 = 32
;; @0020                               v16 = uextend.i64 v8
;; @0020                               v17 = icmp ule v15, v16
;; @0020                               brif v17, block2, block3
;;
;;                                 block2:
;;                                     v66 = iconst.i32 32
;;                                     v64 = iadd.i32 v7, v66  ; v66 = 32
;; @0020                               store notrap aligned vmctx v64, v6
;;                                     v67 = iconst.i32 -1342177280
;;                                     v68 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v69 = load.i64 notrap aligned readonly can_move v68+32
;; @0020                               v31 = iadd v69, v14
;; @0020                               store notrap aligned v67, v31  ; v67 = -1342177280
;;                                     v70 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v71 = load.i32 notrap aligned readonly can_move v70
;; @0020                               store notrap aligned v71, v31+4
;;                                     v72 = iconst.i64 32
;; @0020                               istore32 notrap aligned v72, v31+8  ; v72 = 32
;; @0020                               jump block4(v7, v31)
;;
;;                                 block3 cold:
;; @0020                               v19 = iconst.i32 -1342177280
;; @0020                               v21 = load.i64 notrap aligned readonly can_move v0+40
;; @0020                               v22 = load.i32 notrap aligned readonly can_move v21
;; @0020                               v4 = iconst.i32 32
;; @0020                               v23 = iconst.i32 16
;; @0020                               v24 = call fn0(v0, v19, v22, v4, v23)  ; v19 = -1342177280, v4 = 32, v23 = 16
;; @0020                               v46 = load.i64 notrap aligned readonly can_move v0+8
;; @0020                               v25 = load.i64 notrap aligned readonly can_move v46+32
;; @0020                               v26 = uextend.i64 v24
;; @0020                               v27 = iadd v25, v26
;; @0020                               jump block4(v24, v27)
;;
;;                                 block4(v36: i32, v37: i64):
;;                                     v45 = stack_addr.i64 ss0
;;                                     store notrap v36, v45
;; @0020                               v40 = call fn1(v0, v2), stack_map=[i32 @ ss0+0]
;; @0020                               v41 = ireduce.i32 v40
;;                                     v44 = iconst.i64 16
;; @0020                               v38 = iadd v37, v44  ; v44 = 16
;; @0020                               store user2 little v41, v38
;;                                     v42 = load.i32 notrap v45
;; @0023                               jump block1
;;
;;                                 block1:
;; @0023                               return v42
;; }
