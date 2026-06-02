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
;;                                     v154 = iconst.i64 48
;; @0025                               v24 = iadd v23, v154  ; v154 = 48
;; @0025                               v25 = uextend.i64 v17
;; @0025                               v26 = icmp ule v24, v25
;; @0025                               brif v26, block2, block3
;;
;;                                 block2:
;;                                     v260 = iconst.i32 48
;;                                     v168 = iadd.i32 v16, v260  ; v260 = 48
;; @0025                               store notrap aligned region0 v168, v15
;;                                     v261 = iconst.i32 -1476395002
;;                                     v262 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v263 = load.i64 notrap aligned readonly can_move v262+32
;; @0025                               v40 = iadd v263, v23
;; @0025                               store notrap aligned v261, v40  ; v261 = -1476395002
;;                                     v264 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v265 = load.i32 notrap aligned readonly can_move v264
;; @0025                               store notrap aligned v265, v40+4
;;                                     v266 = iconst.i64 48
;; @0025                               istore32 notrap aligned v266, v40+8  ; v266 = 48
;; @0025                               jump block4(v16, v40)
;;
;;                                 block3 cold:
;; @0025                               v28 = iconst.i32 -1476395002
;; @0025                               v30 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v31 = load.i32 notrap aligned readonly can_move v30
;;                                     v153 = iconst.i32 48
;; @0025                               v32 = iconst.i32 16
;; @0025                               v33 = call fn0(v0, v28, v31, v153, v32)  ; v28 = -1476395002, v153 = 48, v32 = 16
;; @0025                               v139 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v34 = load.i64 notrap aligned readonly can_move v139+32
;; @0025                               v35 = uextend.i64 v33
;; @0025                               v36 = iadd v34, v35
;; @0025                               jump block4(v33, v36)
;;
;;                                 block4(v45: i32, v46: i64):
;; @0025                               v6 = iconst.i32 3
;; @0025                               v47 = iconst.i64 16
;; @0025                               v48 = iadd v46, v47  ; v47 = 16
;; @0025                               store user2 region1 v6, v48  ; v6 = 3
;; @0025                               trapz v45, user16
;;                                     v267 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v268 = load.i64 notrap aligned readonly can_move v267+32
;; @0025                               v50 = uextend.i64 v45
;; @0025                               v52 = iadd v268, v50
;; @0025                               v54 = iadd v52, v47  ; v47 = 16
;; @0025                               v55 = load.i32 user2 readonly region1 v54
;; @0025                               v49 = iconst.i32 0
;;                                     v171 = icmp ne v55, v49  ; v49 = 0
;; @0025                               trapz v171, user17
;; @0025                               v58 = uextend.i64 v55
;;                                     v144 = iconst.i64 3
;;                                     v174 = ishl v58, v144  ; v144 = 3
;;                                     v143 = iconst.i64 32
;; @0025                               v60 = ushr v174, v143  ; v143 = 32
;; @0025                               trapnz v60, user2
;;                                     v183 = ishl v55, v6  ; v6 = 3
;; @0025                               v7 = iconst.i32 24
;; @0025                               v63 = uadd_overflow_trap v183, v7, user2  ; v7 = 24
;; @0025                               v67 = uadd_overflow_trap v45, v63, user2
;; @0025                               v68 = uextend.i64 v67
;; @0025                               v70 = iadd v268, v68
;; @0025                               v71 = isub v63, v7  ; v7 = 24
;; @0025                               v72 = uextend.i64 v71
;; @0025                               v73 = isub v70, v72
;; @0025                               store.i64 user2 little region1 v2, v73
;; @0025                               v80 = load.i32 user2 readonly region1 v54
;; @0025                               v74 = iconst.i32 1
;;                                     v200 = icmp ugt v80, v74  ; v74 = 1
;; @0025                               trapz v200, user17
;; @0025                               v83 = uextend.i64 v80
;;                                     v202 = ishl v83, v144  ; v144 = 3
;; @0025                               v85 = ushr v202, v143  ; v143 = 32
;; @0025                               trapnz v85, user2
;;                                     v209 = ishl v80, v6  ; v6 = 3
;; @0025                               v88 = uadd_overflow_trap v209, v7, user2  ; v7 = 24
;; @0025                               v92 = uadd_overflow_trap v45, v88, user2
;; @0025                               v93 = uextend.i64 v92
;; @0025                               v95 = iadd v268, v93
;;                                     v222 = iconst.i32 32
;; @0025                               v96 = isub v88, v222  ; v222 = 32
;; @0025                               v97 = uextend.i64 v96
;; @0025                               v98 = isub v95, v97
;; @0025                               store.i64 user2 little region1 v3, v98
;; @0025                               v105 = load.i32 user2 readonly region1 v54
;; @0025                               v99 = iconst.i32 2
;;                                     v228 = icmp ugt v105, v99  ; v99 = 2
;; @0025                               trapz v228, user17
;; @0025                               v108 = uextend.i64 v105
;;                                     v230 = ishl v108, v144  ; v144 = 3
;; @0025                               v110 = ushr v230, v143  ; v143 = 32
;; @0025                               trapnz v110, user2
;;                                     v237 = ishl v105, v6  ; v6 = 3
;; @0025                               v113 = uadd_overflow_trap v237, v7, user2  ; v7 = 24
;; @0025                               v117 = uadd_overflow_trap v45, v113, user2
;; @0025                               v118 = uextend.i64 v117
;; @0025                               v120 = iadd v268, v118
;;                                     v254 = iconst.i32 40
;; @0025                               v121 = isub v113, v254  ; v254 = 40
;; @0025                               v122 = uextend.i64 v121
;; @0025                               v123 = isub v120, v122
;; @0025                               store.i64 user2 little region1 v4, v123
;; @0029                               jump block1(v45)
;;
;;                                 block1(v5: i32):
;; @0029                               return v5
;; }
