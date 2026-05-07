;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
;;! test = "optimize"
(module
  (type $ty (array (mut i64)))

  (func (param i64 i32) (result (ref $ty))
    (array.new $ty (local.get 0) (local.get 1))
  )
)
;; function u0:0(i64 vmctx, i64, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0022                               v6 = uextend.i64 v3
;;                                     v67 = iconst.i64 3
;;                                     v68 = ishl v6, v67  ; v67 = 3
;;                                     v65 = iconst.i64 32
;; @0022                               v8 = ushr v68, v65  ; v65 = 32
;; @0022                               trapnz v8, user17
;; @0022                               v5 = iconst.i32 24
;;                                     v74 = iconst.i32 3
;;                                     v75 = ishl v3, v74  ; v74 = 3
;; @0022                               v10 = uadd_overflow_trap v5, v75, user17  ; v5 = 24
;; @0022                               v12 = load.i64 notrap aligned readonly can_move v0+32
;; @0022                               v13 = load.i32 notrap aligned can_move v12
;; @0022                               v20 = uextend.i64 v13
;; @0022                               v15 = uextend.i64 v10
;; @0022                               v16 = iconst.i64 15
;; @0022                               v18 = iadd v15, v16  ; v16 = 15
;; @0022                               v17 = iconst.i64 -16
;; @0022                               v19 = band v18, v17  ; v17 = -16
;; @0022                               v21 = iadd v20, v19
;; @0022                               v14 = load.i32 notrap aligned readonly can_move v12+4
;; @0022                               v22 = uextend.i64 v14
;; @0022                               v23 = icmp ule v21, v22
;; @0022                               brif v23, block2, block3
;;
;;                                 block2:
;;                                     v94 = iconst.i32 15
;;                                     v95 = iadd.i32 v10, v94  ; v94 = 15
;;                                     v98 = iconst.i32 -16
;;                                     v99 = band v95, v98  ; v98 = -16
;;                                     v101 = iadd.i32 v13, v99
;; @0022                               store notrap aligned vmctx v101, v12
;;                                     v105 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v106 = load.i32 notrap aligned readonly can_move v105
;; @0022                               v41 = uextend.i64 v106
;;                                     v107 = iconst.i64 32
;;                                     v108 = ishl v41, v107  ; v107 = 32
;; @0022                               v43 = iconst.i64 0xa800_0000
;;                                     v103 = bor v108, v43  ; v43 = 0xa800_0000
;;                                     v109 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v110 = load.i64 notrap aligned readonly can_move v109+32
;; @0022                               v37 = iadd v110, v20
;; @0022                               store notrap aligned vmctx v103, v37
;; @0022                               store notrap aligned v99, v37+8
;; @0022                               jump block4(v13, v37)
;;
;;                                 block3 cold:
;; @0022                               v25 = iconst.i32 -1476395008
;; @0022                               v39 = load.i64 notrap aligned readonly can_move v0+40
;; @0022                               v40 = load.i32 notrap aligned readonly can_move v39
;; @0022                               v29 = iconst.i32 16
;; @0022                               v30 = call fn0(v0, v25, v40, v10, v29)  ; v25 = -1476395008, v29 = 16
;; @0022                               v63 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v35 = load.i64 notrap aligned readonly can_move v63+32
;; @0022                               v32 = uextend.i64 v30
;; @0022                               v33 = iadd v35, v32
;; @0022                               jump block4(v30, v33)
;;
;;                                 block4(v46: i32, v47: i64):
;;                                     v59 = iconst.i64 16
;; @0022                               v48 = iadd v47, v59  ; v59 = 16
;; @0022                               store.i32 notrap aligned v3, v48
;;                                     v79 = iconst.i64 24
;;                                     v84 = iadd v47, v79  ; v79 = 24
;; @0022                               v55 = iadd v47, v15
;;                                     v66 = iconst.i64 8
;; @0022                               jump block5(v84)
;;
;;                                 block5(v56: i64):
;; @0022                               v57 = icmp eq v56, v55
;; @0022                               brif v57, block7, block6
;;
;;                                 block6:
;; @0022                               store.i64 notrap aligned little v2, v56
;;                                     v111 = iconst.i64 8
;;                                     v112 = iadd.i64 v56, v111  ; v111 = 8
;; @0022                               jump block5(v112)
;;
;;                                 block7:
;; @0025                               jump block1(v46)
;;
;;                                 block1(v4: i32):
;; @0025                               return v4
;; }
