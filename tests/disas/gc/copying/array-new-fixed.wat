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
;;     region0 = 32 "VMContext+0x20"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;; @0025                               v15 = load.i64 notrap aligned readonly can_move v0+32
;; @0025                               v16 = load.i32 notrap aligned v15
;; @0025                               v17 = load.i32 notrap aligned v15+4
;; @0025                               v23 = uextend.i64 v16
;;                                     v150 = iconst.i64 48
;; @0025                               v24 = iadd v23, v150  ; v150 = 48
;; @0025                               v25 = uextend.i64 v17
;; @0025                               v26 = icmp ule v24, v25
;; @0025                               brif v26, block2, block3
;;
;;                                 block2:
;;                                     v256 = iconst.i32 48
;;                                     v164 = iadd.i32 v16, v256  ; v256 = 48
;; @0025                               store notrap aligned region0 v164, v15
;;                                     v257 = iconst.i32 -1476395002
;;                                     v258 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v259 = load.i64 notrap aligned readonly can_move v258+32
;; @0025                               v38 = iadd v259, v23
;; @0025                               store notrap aligned v257, v38  ; v257 = -1476395002
;;                                     v260 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v261 = load.i32 notrap aligned readonly can_move v260
;; @0025                               store notrap aligned v261, v38+4
;;                                     v262 = iconst.i64 48
;; @0025                               istore32 notrap aligned v262, v38+8  ; v262 = 48
;; @0025                               jump block4(v16, v38)
;;
;;                                 block3 cold:
;; @0025                               v27 = iconst.i32 -1476395002
;; @0025                               v28 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v29 = load.i32 notrap aligned readonly can_move v28
;;                                     v149 = iconst.i32 48
;; @0025                               v30 = iconst.i32 16
;; @0025                               v31 = call fn0(v0, v27, v29, v149, v30)  ; v27 = -1476395002, v149 = 48, v30 = 16
;; @0025                               v136 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v32 = load.i64 notrap aligned readonly can_move v136+32
;; @0025                               v33 = uextend.i64 v31
;; @0025                               v34 = iadd v32, v33
;; @0025                               jump block4(v31, v34)
;;
;;                                 block4(v42: i32, v43: i64):
;; @0025                               v6 = iconst.i32 3
;; @0025                               v44 = iconst.i64 16
;; @0025                               v45 = iadd v43, v44  ; v44 = 16
;; @0025                               store user2 region1 v6, v45  ; v6 = 3
;; @0025                               trapz v42, user16
;;                                     v263 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v264 = load.i64 notrap aligned readonly can_move v263+32
;; @0025                               v47 = uextend.i64 v42
;; @0025                               v49 = iadd v264, v47
;; @0025                               v51 = iadd v49, v44  ; v44 = 16
;; @0025                               v52 = load.i32 user2 readonly region1 v51
;; @0025                               trapz v52, user17
;; @0025                               v55 = uextend.i64 v52
;;                                     v140 = iconst.i64 3
;;                                     v170 = ishl v55, v140  ; v140 = 3
;; @0025                               v11 = iconst.i64 32
;; @0025                               v58 = ushr v170, v11  ; v11 = 32
;; @0025                               trapnz v58, user2
;;                                     v179 = ishl v52, v6  ; v6 = 3
;; @0025                               v7 = iconst.i32 24
;; @0025                               v61 = uadd_overflow_trap v179, v7, user2  ; v7 = 24
;; @0025                               v65 = uadd_overflow_trap v42, v61, user2
;; @0025                               v66 = uextend.i64 v65
;; @0025                               v68 = iadd v264, v66
;; @0025                               v69 = isub v61, v7  ; v7 = 24
;; @0025                               v70 = uextend.i64 v69
;; @0025                               v71 = isub v68, v70
;; @0025                               store.i64 user2 little region1 v2, v71
;; @0025                               v78 = load.i32 user2 readonly region1 v51
;; @0025                               v72 = iconst.i32 1
;;                                     v196 = icmp ugt v78, v72  ; v72 = 1
;; @0025                               trapz v196, user17
;; @0025                               v81 = uextend.i64 v78
;;                                     v198 = ishl v81, v140  ; v140 = 3
;; @0025                               v84 = ushr v198, v11  ; v11 = 32
;; @0025                               trapnz v84, user2
;;                                     v205 = ishl v78, v6  ; v6 = 3
;; @0025                               v87 = uadd_overflow_trap v205, v7, user2  ; v7 = 24
;; @0025                               v91 = uadd_overflow_trap v42, v87, user2
;; @0025                               v92 = uextend.i64 v91
;; @0025                               v94 = iadd v264, v92
;;                                     v218 = iconst.i32 32
;; @0025                               v95 = isub v87, v218  ; v218 = 32
;; @0025                               v96 = uextend.i64 v95
;; @0025                               v97 = isub v94, v96
;; @0025                               store.i64 user2 little region1 v3, v97
;; @0025                               v104 = load.i32 user2 readonly region1 v51
;; @0025                               v98 = iconst.i32 2
;;                                     v224 = icmp ugt v104, v98  ; v98 = 2
;; @0025                               trapz v224, user17
;; @0025                               v107 = uextend.i64 v104
;;                                     v226 = ishl v107, v140  ; v140 = 3
;; @0025                               v110 = ushr v226, v11  ; v11 = 32
;; @0025                               trapnz v110, user2
;;                                     v233 = ishl v104, v6  ; v6 = 3
;; @0025                               v113 = uadd_overflow_trap v233, v7, user2  ; v7 = 24
;; @0025                               v117 = uadd_overflow_trap v42, v113, user2
;; @0025                               v118 = uextend.i64 v117
;; @0025                               v120 = iadd v264, v118
;;                                     v250 = iconst.i32 40
;; @0025                               v121 = isub v113, v250  ; v250 = 40
;; @0025                               v122 = uextend.i64 v121
;; @0025                               v123 = isub v120, v122
;; @0025                               store.i64 user2 little region1 v4, v123
;; @0029                               jump block1(v42)
;;
;;                                 block1(v5: i32):
;; @0029                               return v5
;; }
