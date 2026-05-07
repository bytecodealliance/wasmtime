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
;; @0020                               v7 = load.i32 notrap aligned can_move v6
;; @0020                               v14 = uextend.i64 v7
;;                                     v55 = iconst.i64 32
;; @0020                               v15 = iadd v14, v55  ; v55 = 32
;; @0020                               v8 = load.i32 notrap aligned readonly can_move v6+4
;; @0020                               v16 = uextend.i64 v8
;; @0020                               v17 = icmp ule v15, v16
;; @0020                               brif v17, block2, block3
;;
;;                                 block2:
;;                                     v73 = iconst.i32 32
;;                                     v69 = iadd.i32 v7, v73  ; v73 = 32
;; @0020                               store notrap aligned vmctx v69, v6
;;                                     v74 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v75 = load.i32 notrap aligned readonly can_move v74
;; @0020                               v35 = uextend.i64 v75
;;                                     v76 = iconst.i64 32
;;                                     v77 = ishl v35, v76  ; v76 = 32
;; @0020                               v37 = iconst.i64 0xb000_0000
;;                                     v71 = bor v77, v37  ; v37 = 0xb000_0000
;;                                     v78 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v79 = load.i64 notrap aligned readonly can_move v78+32
;; @0020                               v31 = iadd v79, v14
;; @0020                               store notrap aligned vmctx v71, v31
;; @0020                               store notrap aligned v73, v31+8  ; v73 = 32
;; @0020                               jump block4(v7, v31)
;;
;;                                 block3 cold:
;; @0020                               v19 = iconst.i32 -1342177280
;; @0020                               v33 = load.i64 notrap aligned readonly can_move v0+40
;; @0020                               v34 = load.i32 notrap aligned readonly can_move v33
;; @0020                               v4 = iconst.i32 32
;; @0020                               v23 = iconst.i32 16
;; @0020                               v24 = call fn0(v0, v19, v34, v4, v23)  ; v19 = -1342177280, v4 = 32, v23 = 16
;; @0020                               v53 = load.i64 notrap aligned readonly can_move v0+8
;; @0020                               v29 = load.i64 notrap aligned readonly can_move v53+32
;; @0020                               v26 = uextend.i64 v24
;; @0020                               v27 = iadd v29, v26
;; @0020                               jump block4(v24, v27)
;;
;;                                 block4(v40: i32, v41: i64):
;;                                     v49 = stack_addr.i64 ss0
;;                                     store notrap v40, v49
;; @0020                               v44 = call fn1(v0, v2), stack_map=[i32 @ ss0+0]
;; @0020                               v45 = ireduce.i32 v44
;;                                     v48 = iconst.i64 16
;; @0020                               v42 = iadd v41, v48  ; v48 = 16
;; @0020                               store notrap aligned little v45, v42
;;                                     v46 = load.i32 notrap v49
;; @0023                               jump block1
;;
;;                                 block1:
;; @0023                               return v46
;; }
