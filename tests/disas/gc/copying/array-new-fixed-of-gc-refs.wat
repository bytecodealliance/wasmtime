;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
;;! test = "optimize"
(module
  (type $ty (array (mut anyref)))

  (func (param anyref anyref anyref) (result (ref $ty))
    (array.new_fixed $ty 3 (local.get 0) (local.get 1) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     ss1 = explicit_slot 4, align = 4
;;     ss2 = explicit_slot 4, align = 4
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
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v69 = stack_addr.i64 ss2
;;                                     store notrap v2, v69
;;                                     v68 = stack_addr.i64 ss1
;;                                     store notrap v3, v68
;;                                     v67 = stack_addr.i64 ss0
;;                                     store notrap v4, v67
;; @0025                               v14 = load.i64 notrap aligned readonly can_move v0+32
;; @0025                               v15 = load.i32 notrap aligned can_move v14
;; @0025                               v22 = uextend.i64 v15
;;                                     v65 = iconst.i64 32
;; @0025                               v23 = iadd v22, v65  ; v65 = 32
;; @0025                               v16 = load.i32 notrap aligned readonly can_move v14+4
;; @0025                               v24 = uextend.i64 v16
;; @0025                               v25 = icmp ule v23, v24
;; @0025                               brif v25, block2, block3
;;
;;                                 block2:
;;                                     v189 = iconst.i32 32
;;                                     v95 = iadd.i32 v15, v189  ; v189 = 32
;; @0025                               store notrap aligned vmctx v95, v14
;;                                     v190 = iconst.i32 -1476395008
;;                                     v191 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v192 = load.i64 notrap aligned readonly can_move v191+32
;; @0025                               v39 = iadd v192, v22
;; @0025                               store notrap aligned v190, v39  ; v190 = -1476395008
;;                                     v193 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v194 = load.i32 notrap aligned readonly can_move v193
;; @0025                               store notrap aligned v194, v39+4
;;                                     v195 = iconst.i64 32
;; @0025                               istore32 notrap aligned v195, v39+8  ; v195 = 32
;; @0025                               jump block4(v15, v39)
;;
;;                                 block3 cold:
;; @0025                               v27 = iconst.i32 -1476395008
;; @0025                               v29 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v30 = load.i32 notrap aligned readonly can_move v29
;;                                     v81 = iconst.i32 32
;; @0025                               v31 = iconst.i32 16
;; @0025                               v32 = call fn0(v0, v27, v30, v81, v31), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v27 = -1476395008, v81 = 32, v31 = 16
;; @0025                               v61 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v33 = load.i64 notrap aligned readonly can_move v61+32
;; @0025                               v34 = uextend.i64 v32
;; @0025                               v35 = iadd v33, v34
;; @0025                               jump block4(v32, v35)
;;
;;                                 block4(v44: i32, v45: i64):
;; @0025                               v6 = iconst.i32 3
;;                                     v60 = iconst.i64 16
;; @0025                               v46 = iadd v45, v60  ; v60 = 16
;; @0025                               store notrap aligned v6, v46  ; v6 = 3
;;                                     v56 = load.i32 notrap v69
;;                                     v98 = iconst.i64 20
;;                                     v103 = iadd v45, v98  ; v98 = 20
;; @0025                               store notrap aligned little v56, v103
;;                                     v55 = load.i32 notrap v68
;;                                     v106 = iconst.i64 24
;;                                     v111 = iadd v45, v106  ; v106 = 24
;; @0025                               store notrap aligned little v55, v111
;;                                     v54 = load.i32 notrap v67
;;                                     v127 = iconst.i64 28
;;                                     v132 = iadd v45, v127  ; v127 = 28
;; @0025                               store notrap aligned little v54, v132
;; @0029                               jump block1(v44)
;;
;;                                 block1(v5: i32):
;; @0029                               return v5
;; }
