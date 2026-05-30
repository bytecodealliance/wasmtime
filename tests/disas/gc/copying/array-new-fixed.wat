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
;; @0025                               v16 = load.i64 notrap aligned readonly can_move v0+32
;; @0025                               v17 = load.i32 notrap aligned v16
;; @0025                               v18 = load.i32 notrap aligned v16+4
;; @0025                               v24 = uextend.i64 v17
;;                                     v154 = iconst.i64 48
;; @0025                               v25 = iadd v24, v154  ; v154 = 48
;; @0025                               v26 = uextend.i64 v18
;; @0025                               v27 = icmp ule v25, v26
;; @0025                               brif v27, block2, block3
;;
;;                                 block2:
;;                                     v260 = iconst.i32 48
;;                                     v168 = iadd.i32 v17, v260  ; v260 = 48
;; @0025                               store notrap aligned region0 v168, v16
;;                                     v261 = iconst.i32 -1476395002
;;                                     v262 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v263 = load.i64 notrap aligned readonly can_move v262+32
;; @0025                               v41 = iadd v263, v24
;; @0025                               store notrap aligned v261, v41  ; v261 = -1476395002
;;                                     v264 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v265 = load.i32 notrap aligned readonly can_move v264
;; @0025                               store notrap aligned v265, v41+4
;;                                     v266 = iconst.i64 48
;; @0025                               istore32 notrap aligned v266, v41+8  ; v266 = 48
;; @0025                               jump block4(v17, v41)
;;
;;                                 block3 cold:
;; @0025                               v29 = iconst.i32 -1476395002
;; @0025                               v31 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v32 = load.i32 notrap aligned readonly can_move v31
;;                                     v153 = iconst.i32 48
;; @0025                               v33 = iconst.i32 16
;; @0025                               v34 = call fn0(v0, v29, v32, v153, v33)  ; v29 = -1476395002, v153 = 48, v33 = 16
;; @0025                               v140 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v35 = load.i64 notrap aligned readonly can_move v140+32
;; @0025                               v36 = uextend.i64 v34
;; @0025                               v37 = iadd v35, v36
;; @0025                               jump block4(v34, v37)
;;
;;                                 block4(v46: i32, v47: i64):
;; @0025                               v6 = iconst.i32 3
;; @0025                               v48 = iconst.i64 16
;; @0025                               v49 = iadd v47, v48  ; v48 = 16
;; @0025                               store user2 region1 v6, v49  ; v6 = 3
;; @0025                               trapz v46, user16
;;                                     v267 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v268 = load.i64 notrap aligned readonly can_move v267+32
;; @0025                               v51 = uextend.i64 v46
;; @0025                               v53 = iadd v268, v51
;; @0025                               v55 = iadd v53, v48  ; v48 = 16
;; @0025                               v56 = load.i32 user2 readonly region1 v55
;; @0025                               v50 = iconst.i32 0
;;                                     v171 = icmp ne v56, v50  ; v50 = 0
;; @0025                               trapz v171, user17
;; @0025                               v59 = uextend.i64 v56
;;                                     v144 = iconst.i64 3
;;                                     v174 = ishl v59, v144  ; v144 = 3
;; @0025                               v11 = iconst.i64 32
;; @0025                               v62 = ushr v174, v11  ; v11 = 32
;; @0025                               trapnz v62, user2
;;                                     v183 = ishl v56, v6  ; v6 = 3
;; @0025                               v7 = iconst.i32 24
;; @0025                               v65 = uadd_overflow_trap v183, v7, user2  ; v7 = 24
;; @0025                               v69 = uadd_overflow_trap v46, v65, user2
;; @0025                               v70 = uextend.i64 v69
;; @0025                               v72 = iadd v268, v70
;; @0025                               v73 = isub v65, v7  ; v7 = 24
;; @0025                               v74 = uextend.i64 v73
;; @0025                               v75 = isub v72, v74
;; @0025                               store.i64 user2 little region1 v2, v75
;; @0025                               v82 = load.i32 user2 readonly region1 v55
;; @0025                               v76 = iconst.i32 1
;;                                     v200 = icmp ugt v82, v76  ; v76 = 1
;; @0025                               trapz v200, user17
;; @0025                               v85 = uextend.i64 v82
;;                                     v202 = ishl v85, v144  ; v144 = 3
;; @0025                               v88 = ushr v202, v11  ; v11 = 32
;; @0025                               trapnz v88, user2
;;                                     v209 = ishl v82, v6  ; v6 = 3
;; @0025                               v91 = uadd_overflow_trap v209, v7, user2  ; v7 = 24
;; @0025                               v95 = uadd_overflow_trap v46, v91, user2
;; @0025                               v96 = uextend.i64 v95
;; @0025                               v98 = iadd v268, v96
;;                                     v222 = iconst.i32 32
;; @0025                               v99 = isub v91, v222  ; v222 = 32
;; @0025                               v100 = uextend.i64 v99
;; @0025                               v101 = isub v98, v100
;; @0025                               store.i64 user2 little region1 v3, v101
;; @0025                               v108 = load.i32 user2 readonly region1 v55
;; @0025                               v102 = iconst.i32 2
;;                                     v228 = icmp ugt v108, v102  ; v102 = 2
;; @0025                               trapz v228, user17
;; @0025                               v111 = uextend.i64 v108
;;                                     v230 = ishl v111, v144  ; v144 = 3
;; @0025                               v114 = ushr v230, v11  ; v11 = 32
;; @0025                               trapnz v114, user2
;;                                     v237 = ishl v108, v6  ; v6 = 3
;; @0025                               v117 = uadd_overflow_trap v237, v7, user2  ; v7 = 24
;; @0025                               v121 = uadd_overflow_trap v46, v117, user2
;; @0025                               v122 = uextend.i64 v121
;; @0025                               v124 = iadd v268, v122
;;                                     v254 = iconst.i32 40
;; @0025                               v125 = isub v117, v254  ; v254 = 40
;; @0025                               v126 = uextend.i64 v125
;; @0025                               v127 = isub v124, v126
;; @0025                               store.i64 user2 little region1 v4, v127
;; @0029                               jump block1(v46)
;;
;;                                 block1(v5: i32):
;; @0029                               return v5
;; }
