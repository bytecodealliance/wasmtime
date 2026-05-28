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
;;     region0 = 2 "vmctx"
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
;;                                     v137 = iconst.i64 16
;; @0025                               v46 = iadd v45, v137  ; v137 = 16
;; @0025                               store user2 v6, v46  ; v6 = 3
;; @0025                               trapz v44, user16
;;                                     v267 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v268 = load.i64 notrap aligned readonly can_move v267+32
;; @0025                               v48 = uextend.i64 v44
;; @0025                               v50 = iadd v268, v48
;; @0025                               v52 = iadd v50, v137  ; v137 = 16
;; @0025                               v53 = load.i32 user2 readonly v52
;; @0025                               v47 = iconst.i32 0
;;                                     v171 = icmp ne v53, v47  ; v47 = 0
;; @0025                               trapz v171, user17
;; @0025                               v56 = uextend.i64 v53
;;                                     v144 = iconst.i64 3
;;                                     v174 = ishl v56, v144  ; v144 = 3
;;                                     v142 = iconst.i64 32
;; @0025                               v58 = ushr v174, v142  ; v142 = 32
;; @0025                               trapnz v58, user2
;;                                     v183 = ishl v53, v6  ; v6 = 3
;; @0025                               v7 = iconst.i32 24
;; @0025                               v61 = uadd_overflow_trap v183, v7, user2  ; v7 = 24
;; @0025                               v65 = uadd_overflow_trap v44, v61, user2
;; @0025                               v66 = uextend.i64 v65
;; @0025                               v68 = iadd v268, v66
;; @0025                               v69 = isub v61, v7  ; v7 = 24
;; @0025                               v70 = uextend.i64 v69
;; @0025                               v71 = isub v68, v70
;; @0025                               store.i64 user2 little v2, v71
;; @0025                               v78 = load.i32 user2 readonly v52
;; @0025                               v72 = iconst.i32 1
;;                                     v200 = icmp ugt v78, v72  ; v72 = 1
;; @0025                               trapz v200, user17
;; @0025                               v81 = uextend.i64 v78
;;                                     v202 = ishl v81, v144  ; v144 = 3
;; @0025                               v83 = ushr v202, v142  ; v142 = 32
;; @0025                               trapnz v83, user2
;;                                     v209 = ishl v78, v6  ; v6 = 3
;; @0025                               v86 = uadd_overflow_trap v209, v7, user2  ; v7 = 24
;; @0025                               v90 = uadd_overflow_trap v44, v86, user2
;; @0025                               v91 = uextend.i64 v90
;; @0025                               v93 = iadd v268, v91
;;                                     v222 = iconst.i32 32
;; @0025                               v94 = isub v86, v222  ; v222 = 32
;; @0025                               v95 = uextend.i64 v94
;; @0025                               v96 = isub v93, v95
;; @0025                               store.i64 user2 little v3, v96
;; @0025                               v103 = load.i32 user2 readonly v52
;; @0025                               v97 = iconst.i32 2
;;                                     v228 = icmp ugt v103, v97  ; v97 = 2
;; @0025                               trapz v228, user17
;; @0025                               v106 = uextend.i64 v103
;;                                     v230 = ishl v106, v144  ; v144 = 3
;; @0025                               v108 = ushr v230, v142  ; v142 = 32
;; @0025                               trapnz v108, user2
;;                                     v237 = ishl v103, v6  ; v6 = 3
;; @0025                               v111 = uadd_overflow_trap v237, v7, user2  ; v7 = 24
;; @0025                               v115 = uadd_overflow_trap v44, v111, user2
;; @0025                               v116 = uextend.i64 v115
;; @0025                               v118 = iadd v268, v116
;;                                     v254 = iconst.i32 40
;; @0025                               v119 = isub v111, v254  ; v254 = 40
;; @0025                               v120 = uextend.i64 v119
;; @0025                               v121 = isub v118, v120
;; @0025                               store.i64 user2 little v4, v121
;; @0029                               jump block1(v44)
;;
;;                                 block1(v5: i32):
;; @0029                               return v5
;; }
