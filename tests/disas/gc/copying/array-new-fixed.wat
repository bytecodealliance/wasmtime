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
;; @0025                               v14 = load.i64 notrap aligned readonly can_move v0+32
;; @0025                               v15 = load.i32 notrap aligned v14
;; @0025                               v16 = load.i32 notrap aligned v14+4
;; @0025                               v22 = uextend.i64 v15
;;                                     v154 = iconst.i64 48
;; @0025                               v23 = iadd v22, v154  ; v154 = 48
;; @0025                               v24 = uextend.i64 v16
;; @0025                               v25 = icmp ule v23, v24
;; @0025                               brif v25, block2, block3
;;
;;                                 block2:
;;                                     v260 = iconst.i32 48
;;                                     v168 = iadd.i32 v15, v260  ; v260 = 48
;; @0025                               store notrap aligned region0 v168, v14
;;                                     v261 = iconst.i32 -1476395002
;;                                     v262 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v263 = load.i64 notrap aligned readonly can_move v262+32
;; @0025                               v39 = iadd v263, v22
;; @0025                               store notrap aligned v261, v39  ; v261 = -1476395002
;;                                     v264 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v265 = load.i32 notrap aligned readonly can_move v264
;; @0025                               store notrap aligned v265, v39+4
;;                                     v266 = iconst.i64 48
;; @0025                               istore32 notrap aligned v266, v39+8  ; v266 = 48
;; @0025                               jump block4(v15, v39)
;;
;;                                 block3 cold:
;; @0025                               v27 = iconst.i32 -1476395002
;; @0025                               v29 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v30 = load.i32 notrap aligned readonly can_move v29
;;                                     v153 = iconst.i32 48
;; @0025                               v31 = iconst.i32 16
;; @0025                               v32 = call fn0(v0, v27, v30, v153, v31)  ; v27 = -1476395002, v153 = 48, v31 = 16
;; @0025                               v138 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v33 = load.i64 notrap aligned readonly can_move v138+32
;; @0025                               v34 = uextend.i64 v32
;; @0025                               v35 = iadd v33, v34
;; @0025                               jump block4(v32, v35)
;;
;;                                 block4(v44: i32, v45: i64):
;; @0025                               v6 = iconst.i32 3
;; @0025                               v46 = iconst.i64 16
;; @0025                               v47 = iadd v45, v46  ; v46 = 16
;; @0025                               store user2 region1 v6, v47  ; v6 = 3
;; @0025                               trapz v44, user16
;;                                     v267 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v268 = load.i64 notrap aligned readonly can_move v267+32
;; @0025                               v49 = uextend.i64 v44
;; @0025                               v51 = iadd v268, v49
;; @0025                               v53 = iadd v51, v46  ; v46 = 16
;; @0025                               v54 = load.i32 user2 readonly region1 v53
;; @0025                               v48 = iconst.i32 0
;;                                     v171 = icmp ne v54, v48  ; v48 = 0
;; @0025                               trapz v171, user17
;; @0025                               v57 = uextend.i64 v54
;;                                     v144 = iconst.i64 3
;;                                     v174 = ishl v57, v144  ; v144 = 3
;;                                     v142 = iconst.i64 32
;; @0025                               v59 = ushr v174, v142  ; v142 = 32
;; @0025                               trapnz v59, user2
;;                                     v183 = ishl v54, v6  ; v6 = 3
;; @0025                               v7 = iconst.i32 24
;; @0025                               v62 = uadd_overflow_trap v183, v7, user2  ; v7 = 24
;; @0025                               v66 = uadd_overflow_trap v44, v62, user2
;; @0025                               v67 = uextend.i64 v66
;; @0025                               v69 = iadd v268, v67
;; @0025                               v70 = isub v62, v7  ; v7 = 24
;; @0025                               v71 = uextend.i64 v70
;; @0025                               v72 = isub v69, v71
;; @0025                               store.i64 user2 little region1 v2, v72
;; @0025                               v79 = load.i32 user2 readonly region1 v53
;; @0025                               v73 = iconst.i32 1
;;                                     v200 = icmp ugt v79, v73  ; v73 = 1
;; @0025                               trapz v200, user17
;; @0025                               v82 = uextend.i64 v79
;;                                     v202 = ishl v82, v144  ; v144 = 3
;; @0025                               v84 = ushr v202, v142  ; v142 = 32
;; @0025                               trapnz v84, user2
;;                                     v209 = ishl v79, v6  ; v6 = 3
;; @0025                               v87 = uadd_overflow_trap v209, v7, user2  ; v7 = 24
;; @0025                               v91 = uadd_overflow_trap v44, v87, user2
;; @0025                               v92 = uextend.i64 v91
;; @0025                               v94 = iadd v268, v92
;;                                     v222 = iconst.i32 32
;; @0025                               v95 = isub v87, v222  ; v222 = 32
;; @0025                               v96 = uextend.i64 v95
;; @0025                               v97 = isub v94, v96
;; @0025                               store.i64 user2 little region1 v3, v97
;; @0025                               v104 = load.i32 user2 readonly region1 v53
;; @0025                               v98 = iconst.i32 2
;;                                     v228 = icmp ugt v104, v98  ; v98 = 2
;; @0025                               trapz v228, user17
;; @0025                               v107 = uextend.i64 v104
;;                                     v230 = ishl v107, v144  ; v144 = 3
;; @0025                               v109 = ushr v230, v142  ; v142 = 32
;; @0025                               trapnz v109, user2
;;                                     v237 = ishl v104, v6  ; v6 = 3
;; @0025                               v112 = uadd_overflow_trap v237, v7, user2  ; v7 = 24
;; @0025                               v116 = uadd_overflow_trap v44, v112, user2
;; @0025                               v117 = uextend.i64 v116
;; @0025                               v119 = iadd v268, v117
;;                                     v254 = iconst.i32 40
;; @0025                               v120 = isub v112, v254  ; v254 = 40
;; @0025                               v121 = uextend.i64 v120
;; @0025                               v122 = isub v119, v121
;; @0025                               store.i64 user2 little region1 v4, v122
;; @0029                               jump block1(v44)
;;
;;                                 block1(v5: i32):
;; @0029                               return v5
;; }
