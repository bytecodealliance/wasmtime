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
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v130 = stack_addr.i64 ss2
;;                                     store notrap v2, v130
;;                                     v131 = stack_addr.i64 ss1
;;                                     store notrap v3, v131
;;                                     v132 = stack_addr.i64 ss0
;;                                     store notrap v4, v132
;;                                     v170 = iconst.i64 0
;; @0025                               trapnz v170, user18  ; v170 = 0
;; @0025                               v7 = iconst.i32 28
;;                                     v171 = iconst.i32 12
;; @0025                               v12 = uadd_overflow_trap v7, v171, user18  ; v7 = 28, v171 = 12
;;                                     v172 = iconst.i32 -1476395005
;; @0025                               v16 = iconst.i32 0
;; @0025                               v17 = iconst.i32 8
;; @0025                               v18 = call fn0(v0, v172, v16, v12, v17), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v172 = -1476395005, v16 = 0, v17 = 8
;; @0025                               v6 = iconst.i32 3
;; @0025                               v20 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v21 = uextend.i64 v18
;; @0025                               v22 = iadd v20, v21
;;                                     v135 = iconst.i64 24
;; @0025                               v23 = iadd v22, v135  ; v135 = 24
;; @0025                               store notrap aligned v6, v23  ; v6 = 3
;;                                     v129 = load.i32 notrap v130
;;                                     v137 = iconst.i32 1
;; @0025                               v28 = band v129, v137  ; v137 = 1
;; @0025                               v29 = icmp eq v129, v16  ; v16 = 0
;; @0025                               v30 = uextend.i32 v29
;; @0025                               v31 = bor v28, v30
;; @0025                               brif v31, block3, block2
;;
;;                                 block2:
;; @0025                               v36 = uextend.i64 v129
;; @0025                               v95 = iconst.i64 8
;; @0025                               v38 = uadd_overflow_trap v36, v95, user1  ; v95 = 8
;; @0025                               v40 = uadd_overflow_trap v38, v95, user1  ; v95 = 8
;; @0025                               v93 = load.i64 notrap aligned readonly can_move v0+48
;; @0025                               v41 = icmp ule v40, v93
;; @0025                               trapz v41, user1
;; @0025                               v42 = iadd.i64 v20, v38
;; @0025                               v43 = load.i64 notrap aligned v42
;;                                     v157 = iconst.i64 1
;; @0025                               v44 = iadd v43, v157  ; v157 = 1
;; @0025                               store notrap aligned v44, v42
;; @0025                               jump block3
;;
;;                                 block3:
;;                                     v125 = load.i32 notrap v130
;;                                     v180 = iconst.i64 28
;;                                     v186 = iadd.i64 v22, v180  ; v180 = 28
;; @0025                               store notrap aligned little v125, v186
;;                                     v124 = load.i32 notrap v131
;;                                     v210 = iconst.i32 1
;;                                     v211 = band v124, v210  ; v210 = 1
;;                                     v212 = iconst.i32 0
;;                                     v213 = icmp eq v124, v212  ; v212 = 0
;; @0025                               v59 = uextend.i32 v213
;; @0025                               v60 = bor v211, v59
;; @0025                               brif v60, block5, block4
;;
;;                                 block4:
;; @0025                               v65 = uextend.i64 v124
;;                                     v214 = iconst.i64 8
;; @0025                               v67 = uadd_overflow_trap v65, v214, user1  ; v214 = 8
;; @0025                               v69 = uadd_overflow_trap v67, v214, user1  ; v214 = 8
;;                                     v215 = load.i64 notrap aligned readonly can_move v0+48
;; @0025                               v70 = icmp ule v69, v215
;; @0025                               trapz v70, user1
;; @0025                               v71 = iadd.i64 v20, v67
;; @0025                               v72 = load.i64 notrap aligned v71
;;                                     v216 = iconst.i64 1
;; @0025                               v73 = iadd v72, v216  ; v216 = 1
;; @0025                               store notrap aligned v73, v71
;; @0025                               jump block5
;;
;;                                 block5:
;;                                     v120 = load.i32 notrap v131
;;                                     v134 = iconst.i64 32
;;                                     v193 = iadd.i64 v22, v134  ; v134 = 32
;; @0025                               store notrap aligned little v120, v193
;;                                     v119 = load.i32 notrap v132
;;                                     v217 = iconst.i32 1
;;                                     v218 = band v119, v217  ; v217 = 1
;;                                     v219 = iconst.i32 0
;;                                     v220 = icmp eq v119, v219  ; v219 = 0
;; @0025                               v88 = uextend.i32 v220
;; @0025                               v89 = bor v218, v88
;; @0025                               brif v89, block7, block6
;;
;;                                 block6:
;; @0025                               v94 = uextend.i64 v119
;;                                     v221 = iconst.i64 8
;; @0025                               v96 = uadd_overflow_trap v94, v221, user1  ; v221 = 8
;; @0025                               v98 = uadd_overflow_trap v96, v221, user1  ; v221 = 8
;;                                     v222 = load.i64 notrap aligned readonly can_move v0+48
;; @0025                               v99 = icmp ule v98, v222
;; @0025                               trapz v99, user1
;; @0025                               v100 = iadd.i64 v20, v96
;; @0025                               v101 = load.i64 notrap aligned v100
;;                                     v223 = iconst.i64 1
;; @0025                               v102 = iadd v101, v223  ; v223 = 1
;; @0025                               store notrap aligned v102, v100
;; @0025                               jump block7
;;
;;                                 block7:
;;                                     v115 = load.i32 notrap v132
;;                                     v195 = iconst.i64 36
;;                                     v201 = iadd.i64 v22, v195  ; v195 = 36
;; @0025                               store notrap aligned little v115, v201
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v18
;; }
