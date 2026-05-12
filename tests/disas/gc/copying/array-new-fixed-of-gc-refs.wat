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
;;                                     v73 = stack_addr.i64 ss2
;;                                     store notrap v2, v73
;;                                     v72 = stack_addr.i64 ss1
;;                                     store notrap v3, v72
;;                                     v71 = stack_addr.i64 ss0
;;                                     store notrap v4, v71
;; @0025                               v14 = load.i64 notrap aligned readonly can_move v0+32
;; @0025                               v15 = load.i32 notrap aligned can_move v14
;; @0025                               v22 = uextend.i64 v15
;;                                     v69 = iconst.i64 32
;; @0025                               v23 = iadd v22, v69  ; v69 = 32
;; @0025                               v16 = load.i32 notrap aligned readonly can_move v14+4
;; @0025                               v24 = uextend.i64 v16
;; @0025                               v25 = icmp ule v23, v24
;; @0025                               brif v25, block2, block3
;;
;;                                 block2:
;;                                     v195 = iconst.i32 32
;;                                     v191 = iadd.i32 v15, v195  ; v195 = 32
;; @0025                               store notrap aligned vmctx v191, v14
;;                                     v196 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v197 = load.i32 notrap aligned readonly can_move v196
;; @0025                               v43 = uextend.i64 v197
;;                                     v198 = iconst.i64 32
;;                                     v199 = ishl v43, v198  ; v198 = 32
;; @0025                               v45 = iconst.i64 0xa800_0000
;;                                     v193 = bor v199, v45  ; v45 = 0xa800_0000
;;                                     v200 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v201 = load.i64 notrap aligned readonly can_move v200+32
;; @0025                               v39 = iadd v201, v22
;; @0025                               store notrap aligned vmctx v193, v39
;; @0025                               istore32 notrap aligned v198, v39+8  ; v198 = 32
;; @0025                               jump block4(v15, v39)
;;
;;                                 block3 cold:
;; @0025                               v27 = iconst.i32 -1476395008
;; @0025                               v41 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v42 = load.i32 notrap aligned readonly can_move v41
;;                                     v85 = iconst.i32 32
;; @0025                               v31 = iconst.i32 16
;; @0025                               v32 = call fn0(v0, v27, v42, v85, v31), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v27 = -1476395008, v85 = 32, v31 = 16
;; @0025                               v67 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v37 = load.i64 notrap aligned readonly can_move v67+32
;; @0025                               v34 = uextend.i64 v32
;; @0025                               v35 = iadd v37, v34
;; @0025                               jump block4(v32, v35)
;;
;;                                 block4(v47: i32, v48: i64):
;; @0025                               v6 = iconst.i32 3
;;                                     v63 = iconst.i64 16
;; @0025                               v49 = iadd v48, v63  ; v63 = 16
;; @0025                               store notrap aligned v6, v49  ; v6 = 3
;;                                     v59 = load.i32 notrap v73
;;                                     v98 = iconst.i64 20
;;                                     v103 = iadd v48, v98  ; v98 = 20
;; @0025                               store notrap aligned little v59, v103
;;                                     v58 = load.i32 notrap v72
;;                                     v106 = iconst.i64 24
;;                                     v111 = iadd v48, v106  ; v106 = 24
;; @0025                               store notrap aligned little v58, v111
;;                                     v57 = load.i32 notrap v71
;;                                     v127 = iconst.i64 28
;;                                     v132 = iadd v48, v127  ; v127 = 28
;; @0025                               store notrap aligned little v57, v132
;; @0029                               jump block1(v47)
;;
;;                                 block1(v5: i32):
;; @0029                               return v5
;; }
