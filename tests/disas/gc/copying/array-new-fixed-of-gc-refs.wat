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
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v152 = stack_addr.i64 ss2
;;                                     store notrap v2, v152
;;                                     v151 = stack_addr.i64 ss1
;;                                     store notrap v3, v151
;;                                     v150 = stack_addr.i64 ss0
;;                                     store notrap v4, v150
;; @0025                               v14 = load.i64 notrap aligned readonly can_move v0+32
;; @0025                               v15 = load.i32 notrap aligned v14
;; @0025                               v16 = load.i32 notrap aligned v14+4
;; @0025                               v22 = uextend.i64 v15
;;                                     v148 = iconst.i64 32
;; @0025                               v23 = iadd v22, v148  ; v148 = 32
;; @0025                               v24 = uextend.i64 v16
;; @0025                               v25 = icmp ule v23, v24
;; @0025                               brif v25, block2, block3
;;
;;                                 block2:
;;                                     v272 = iconst.i32 32
;;                                     v178 = iadd.i32 v15, v272  ; v272 = 32
;; @0025                               store notrap aligned region0 v178, v14
;;                                     v273 = iconst.i32 -1476394994
;;                                     v274 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v275 = load.i64 notrap aligned readonly can_move v274+32
;; @0025                               v39 = iadd v275, v22
;; @0025                               store notrap aligned v273, v39  ; v273 = -1476394994
;;                                     v276 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v277 = load.i32 notrap aligned readonly can_move v276
;; @0025                               store notrap aligned v277, v39+4
;;                                     v278 = iconst.i64 32
;; @0025                               istore32 notrap aligned v278, v39+8  ; v278 = 32
;; @0025                               jump block4(v15, v39)
;;
;;                                 block3 cold:
;; @0025                               v27 = iconst.i32 -1476394994
;; @0025                               v29 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v30 = load.i32 notrap aligned readonly can_move v29
;;                                     v164 = iconst.i32 32
;; @0025                               v31 = iconst.i32 16
;; @0025                               v32 = call fn0(v0, v27, v30, v164, v31), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v27 = -1476394994, v164 = 32, v31 = 16
;; @0025                               v144 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v33 = load.i64 notrap aligned readonly can_move v144+32
;; @0025                               v34 = uextend.i64 v32
;; @0025                               v35 = iadd v33, v34
;; @0025                               jump block4(v32, v35)
;;
;;                                 block4(v44: i32, v45: i64):
;; @0025                               v6 = iconst.i32 3
;;                                     v143 = iconst.i64 16
;; @0025                               v46 = iadd v45, v143  ; v143 = 16
;; @0025                               store user2 v6, v46  ; v6 = 3
;; @0025                               trapz v44, user16
;;                                     v279 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v280 = load.i64 notrap aligned readonly can_move v279+32
;; @0025                               v48 = uextend.i64 v44
;; @0025                               v50 = iadd v280, v48
;; @0025                               v52 = iadd v50, v143  ; v143 = 16
;; @0025                               v53 = load.i32 user2 readonly v52
;; @0025                               v47 = iconst.i32 0
;;                                     v181 = icmp ne v53, v47  ; v47 = 0
;; @0025                               trapz v181, user17
;; @0025                               v56 = uextend.i64 v53
;;                                     v155 = iconst.i64 2
;;                                     v184 = ishl v56, v155  ; v155 = 2
;;                                     v281 = iconst.i64 32
;;                                     v282 = ushr v184, v281  ; v281 = 32
;; @0025                               trapnz v282, user2
;;                                     v193 = iconst.i32 2
;;                                     v194 = ishl v53, v193  ; v193 = 2
;; @0025                               v7 = iconst.i32 20
;; @0025                               v61 = uadd_overflow_trap v194, v7, user2  ; v7 = 20
;; @0025                               v65 = uadd_overflow_trap v44, v61, user2
;;                                     v124 = load.i32 notrap v152
;; @0025                               v66 = uextend.i64 v65
;; @0025                               v68 = iadd v280, v66
;; @0025                               v69 = isub v61, v7  ; v7 = 20
;; @0025                               v70 = uextend.i64 v69
;; @0025                               v71 = isub v68, v70
;; @0025                               store user2 little v124, v71
;; @0025                               v78 = load.i32 user2 readonly v52
;; @0025                               v72 = iconst.i32 1
;;                                     v211 = icmp ugt v78, v72  ; v72 = 1
;; @0025                               trapz v211, user17
;; @0025                               v81 = uextend.i64 v78
;;                                     v213 = ishl v81, v155  ; v155 = 2
;;                                     v283 = ushr v213, v281  ; v281 = 32
;; @0025                               trapnz v283, user2
;;                                     v220 = ishl v78, v193  ; v193 = 2
;; @0025                               v86 = uadd_overflow_trap v220, v7, user2  ; v7 = 20
;; @0025                               v90 = uadd_overflow_trap v44, v86, user2
;;                                     v123 = load.i32 notrap v151
;; @0025                               v91 = uextend.i64 v90
;; @0025                               v93 = iadd v280, v91
;;                                     v233 = iconst.i32 24
;; @0025                               v94 = isub v86, v233  ; v233 = 24
;; @0025                               v95 = uextend.i64 v94
;; @0025                               v96 = isub v93, v95
;; @0025                               store user2 little v123, v96
;; @0025                               v103 = load.i32 user2 readonly v52
;;                                     v239 = icmp ugt v103, v193  ; v193 = 2
;; @0025                               trapz v239, user17
;; @0025                               v106 = uextend.i64 v103
;;                                     v241 = ishl v106, v155  ; v155 = 2
;;                                     v284 = ushr v241, v281  ; v281 = 32
;; @0025                               trapnz v284, user2
;;                                     v248 = ishl v103, v193  ; v193 = 2
;; @0025                               v111 = uadd_overflow_trap v248, v7, user2  ; v7 = 20
;; @0025                               v115 = uadd_overflow_trap v44, v111, user2
;;                                     v122 = load.i32 notrap v150
;; @0025                               v116 = uextend.i64 v115
;; @0025                               v118 = iadd v280, v116
;;                                     v266 = iconst.i32 28
;; @0025                               v119 = isub v111, v266  ; v266 = 28
;; @0025                               v120 = uextend.i64 v119
;; @0025                               v121 = isub v118, v120
;; @0025                               store user2 little v122, v121
;; @0029                               jump block1(v44)
;;
;;                                 block1(v5: i32):
;; @0029                               return v5
;; }
