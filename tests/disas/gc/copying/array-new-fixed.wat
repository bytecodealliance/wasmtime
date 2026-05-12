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
;; @0025                               v15 = load.i32 notrap aligned can_move v14
;; @0025                               v22 = uextend.i64 v15
;;                                     v75 = iconst.i64 48
;; @0025                               v23 = iadd v22, v75  ; v75 = 48
;; @0025                               v16 = load.i32 notrap aligned readonly can_move v14+4
;; @0025                               v24 = uextend.i64 v16
;; @0025                               v25 = icmp ule v23, v24
;; @0025                               brif v25, block2, block3
;;
;;                                 block2:
;;                                     v160 = iconst.i32 48
;;                                     v156 = iadd.i32 v15, v160  ; v160 = 48
;; @0025                               store notrap aligned vmctx v156, v14
;;                                     v161 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v162 = load.i32 notrap aligned readonly can_move v161
;; @0025                               v43 = uextend.i64 v162
;;                                     v63 = iconst.i64 32
;; @0025                               v44 = ishl v43, v63  ; v63 = 32
;; @0025                               v45 = iconst.i64 0xa800_0000
;;                                     v158 = bor v44, v45  ; v45 = 0xa800_0000
;;                                     v163 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v164 = load.i64 notrap aligned readonly can_move v163+32
;; @0025                               v39 = iadd v164, v22
;; @0025                               store notrap aligned vmctx v158, v39
;;                                     v165 = iconst.i64 48
;; @0025                               istore32 notrap aligned v165, v39+8  ; v165 = 48
;; @0025                               jump block4(v15, v39)
;;
;;                                 block3 cold:
;; @0025                               v27 = iconst.i32 -1476395008
;; @0025                               v41 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v42 = load.i32 notrap aligned readonly can_move v41
;;                                     v74 = iconst.i32 48
;; @0025                               v31 = iconst.i32 16
;; @0025                               v32 = call fn0(v0, v27, v42, v74, v31)  ; v27 = -1476395008, v74 = 48, v31 = 16
;; @0025                               v61 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v37 = load.i64 notrap aligned readonly can_move v61+32
;; @0025                               v34 = uextend.i64 v32
;; @0025                               v35 = iadd v37, v34
;; @0025                               jump block4(v32, v35)
;;
;;                                 block4(v47: i32, v48: i64):
;; @0025                               v6 = iconst.i32 3
;;                                     v57 = iconst.i64 16
;; @0025                               v49 = iadd v48, v57  ; v57 = 16
;; @0025                               store notrap aligned v6, v49  ; v6 = 3
;;                                     v66 = iconst.i64 24
;;                                     v92 = iadd v48, v66  ; v66 = 24
;; @0025                               store.i64 notrap aligned little v2, v92
;;                                     v166 = iconst.i64 32
;;                                     v99 = iadd v48, v166  ; v166 = 32
;; @0025                               store.i64 notrap aligned little v3, v99
;;                                     v114 = iconst.i64 40
;;                                     v119 = iadd v48, v114  ; v114 = 40
;; @0025                               store.i64 notrap aligned little v4, v119
;; @0029                               jump block1(v47)
;;
;;                                 block1(v5: i32):
;; @0029                               return v5
;; }
