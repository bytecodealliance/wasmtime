;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
;;! test = "optimize"
(module
  (type $ty (array (mut i64)))

  (func (param i64 i64 i64) (result (ref $ty))
    (array.new_fixed $ty 3 (local.get 0) (local.get 1) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, i64, i64, i64) -> i32 tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;; @0025                               v14 = load.i64 notrap aligned readonly can_move v0+32
;; @0025                               v15 = load.i32 notrap aligned v14
;; @0025                               v16 = load.i32 notrap aligned v14+4
;; @0025                               v22 = uextend.i64 v15
;;                                     v71 = iconst.i64 48
;; @0025                               v23 = iadd v22, v71  ; v71 = 48
;; @0025                               v24 = uextend.i64 v16
;; @0025                               v25 = icmp ule v23, v24
;; @0025                               brif v25, block2, block3
;;
;;                                 block2:
;;                                     v154 = iconst.i32 48
;;                                     v85 = iadd.i32 v15, v154  ; v154 = 48
;; @0025                               store notrap aligned vmctx v85, v14
;;                                     v155 = iconst.i32 -1476395008
;;                                     v156 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v157 = load.i64 notrap aligned readonly can_move v156+32
;; @0025                               v39 = iadd v157, v22
;; @0025                               store notrap aligned v155, v39  ; v155 = -1476395008
;;                                     v158 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v159 = load.i32 notrap aligned readonly can_move v158
;; @0025                               store notrap aligned v159, v39+4
;;                                     v160 = iconst.i64 48
;; @0025                               istore32 notrap aligned v160, v39+8  ; v160 = 48
;; @0025                               jump block4(v15, v39)
;;
;;                                 block3 cold:
;; @0025                               v27 = iconst.i32 -1476395008
;; @0025                               v29 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v30 = load.i32 notrap aligned readonly can_move v29
;;                                     v70 = iconst.i32 48
;; @0025                               v31 = iconst.i32 16
;; @0025                               v32 = call fn0(v0, v27, v30, v70, v31)  ; v27 = -1476395008, v70 = 48, v31 = 16
;; @0025                               v55 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v33 = load.i64 notrap aligned readonly can_move v55+32
;; @0025                               v34 = uextend.i64 v32
;; @0025                               v35 = iadd v33, v34
;; @0025                               jump block4(v32, v35)
;;
;;                                 block4(v44: i32, v45: i64):
;; @0025                               v6 = iconst.i32 3
;;                                     v54 = iconst.i64 16
;; @0025                               v46 = iadd v45, v54  ; v54 = 16
;; @0025                               store user2 v6, v46  ; v6 = 3
;;                                     v62 = iconst.i64 24
;;                                     v92 = iadd v45, v62  ; v62 = 24
;; @0025                               store.i64 user2 little v2, v92
;;                                     v59 = iconst.i64 32
;;                                     v99 = iadd v45, v59  ; v59 = 32
;; @0025                               store.i64 user2 little v3, v99
;;                                     v114 = iconst.i64 40
;;                                     v119 = iadd v45, v114  ; v114 = 40
;; @0025                               store.i64 user2 little v4, v119
;; @0029                               jump block1(v44)
;;
;;                                 block1(v5: i32):
;; @0029                               return v5
;; }
