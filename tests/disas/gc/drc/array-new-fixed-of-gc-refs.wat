;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
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
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i64 tail
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v131 = stack_addr.i64 ss2
;;                                     store notrap v2, v131
;;                                     v132 = stack_addr.i64 ss1
;;                                     store notrap v3, v132
;;                                     v133 = stack_addr.i64 ss0
;;                                     store notrap v4, v133
;;                                     v171 = iconst.i64 0
;; @0025                               trapnz v171, user18  ; v171 = 0
;; @0025                               v7 = iconst.i32 28
;;                                     v172 = iconst.i32 12
;; @0025                               v12 = uadd_overflow_trap v7, v172, user18  ; v7 = 28, v172 = 12
;;                                     v173 = iconst.i32 -1476395005
;; @0025                               v16 = iconst.i32 0
;; @0025                               v17 = iconst.i32 8
;; @0025                               v18 = call fn0(v0, v173, v16, v12, v17), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v173 = -1476395005, v16 = 0, v17 = 8
;; @0025                               v6 = iconst.i32 3
;; @0025                               v21 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v19 = ireduce.i32 v18
;; @0025                               v22 = uextend.i64 v19
;; @0025                               v23 = iadd v21, v22
;;                                     v136 = iconst.i64 24
;; @0025                               v24 = iadd v23, v136  ; v136 = 24
;; @0025                               store notrap aligned v6, v24  ; v6 = 3
;;                                     v130 = load.i32 notrap v131
;;                                     v138 = iconst.i32 1
;; @0025                               v29 = band v130, v138  ; v138 = 1
;; @0025                               v30 = icmp eq v130, v16  ; v16 = 0
;; @0025                               v31 = uextend.i32 v30
;; @0025                               v32 = bor v29, v31
;; @0025                               brif v32, block3, block2
;;
;;                                 block2:
;; @0025                               v37 = uextend.i64 v130
;; @0025                               v96 = iconst.i64 8
;; @0025                               v39 = uadd_overflow_trap v37, v96, user1  ; v96 = 8
;; @0025                               v41 = uadd_overflow_trap v39, v96, user1  ; v96 = 8
;; @0025                               v94 = load.i64 notrap aligned readonly can_move v0+48
;; @0025                               v42 = icmp ule v41, v94
;; @0025                               trapz v42, user1
;; @0025                               v43 = iadd.i64 v21, v39
;; @0025                               v44 = load.i64 notrap aligned v43
;;                                     v158 = iconst.i64 1
;; @0025                               v45 = iadd v44, v158  ; v158 = 1
;; @0025                               store notrap aligned v45, v43
;; @0025                               jump block3
;;
;;                                 block3:
;;                                     v126 = load.i32 notrap v131
;;                                     v181 = iconst.i64 28
;;                                     v187 = iadd.i64 v23, v181  ; v181 = 28
;; @0025                               store notrap aligned little v126, v187
;;                                     v125 = load.i32 notrap v132
;;                                     v211 = iconst.i32 1
;;                                     v212 = band v125, v211  ; v211 = 1
;;                                     v213 = iconst.i32 0
;;                                     v214 = icmp eq v125, v213  ; v213 = 0
;; @0025                               v60 = uextend.i32 v214
;; @0025                               v61 = bor v212, v60
;; @0025                               brif v61, block5, block4
;;
;;                                 block4:
;; @0025                               v66 = uextend.i64 v125
;;                                     v215 = iconst.i64 8
;; @0025                               v68 = uadd_overflow_trap v66, v215, user1  ; v215 = 8
;; @0025                               v70 = uadd_overflow_trap v68, v215, user1  ; v215 = 8
;;                                     v216 = load.i64 notrap aligned readonly can_move v0+48
;; @0025                               v71 = icmp ule v70, v216
;; @0025                               trapz v71, user1
;; @0025                               v72 = iadd.i64 v21, v68
;; @0025                               v73 = load.i64 notrap aligned v72
;;                                     v217 = iconst.i64 1
;; @0025                               v74 = iadd v73, v217  ; v217 = 1
;; @0025                               store notrap aligned v74, v72
;; @0025                               jump block5
;;
;;                                 block5:
;;                                     v121 = load.i32 notrap v132
;;                                     v135 = iconst.i64 32
;;                                     v194 = iadd.i64 v23, v135  ; v135 = 32
;; @0025                               store notrap aligned little v121, v194
;;                                     v120 = load.i32 notrap v133
;;                                     v218 = iconst.i32 1
;;                                     v219 = band v120, v218  ; v218 = 1
;;                                     v220 = iconst.i32 0
;;                                     v221 = icmp eq v120, v220  ; v220 = 0
;; @0025                               v89 = uextend.i32 v221
;; @0025                               v90 = bor v219, v89
;; @0025                               brif v90, block7, block6
;;
;;                                 block6:
;; @0025                               v95 = uextend.i64 v120
;;                                     v222 = iconst.i64 8
;; @0025                               v97 = uadd_overflow_trap v95, v222, user1  ; v222 = 8
;; @0025                               v99 = uadd_overflow_trap v97, v222, user1  ; v222 = 8
;;                                     v223 = load.i64 notrap aligned readonly can_move v0+48
;; @0025                               v100 = icmp ule v99, v223
;; @0025                               trapz v100, user1
;; @0025                               v101 = iadd.i64 v21, v97
;; @0025                               v102 = load.i64 notrap aligned v101
;;                                     v224 = iconst.i64 1
;; @0025                               v103 = iadd v102, v224  ; v224 = 1
;; @0025                               store notrap aligned v103, v101
;; @0025                               jump block7
;;
;;                                 block7:
;;                                     v116 = load.i32 notrap v133
;;                                     v196 = iconst.i64 36
;;                                     v202 = iadd.i64 v23, v196  ; v196 = 36
;; @0025                               store notrap aligned little v116, v202
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v19
;; }
