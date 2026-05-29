;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
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
;;     region0 = 32 "VMContext+0x20"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned gv4+40
;;     gv6 = load.i64 notrap aligned readonly can_move gv4+32
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     fn0 = colocated u805306368:23 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v145 = stack_addr.i64 ss2
;;                                     store notrap v2, v145
;;                                     v144 = stack_addr.i64 ss1
;;                                     store notrap v3, v144
;;                                     v143 = stack_addr.i64 ss0
;;                                     store notrap v4, v143
;; @0025                               v17 = load.i64 notrap aligned readonly region0 v0+32
;; @0025                               v18 = load.i32 user2 region1 v17
;;                                     v163 = iconst.i32 7
;; @0025                               v21 = uadd_overflow_trap v18, v163, user18  ; v163 = 7
;;                                     v169 = iconst.i32 -8
;; @0025                               v23 = band v21, v169  ; v169 = -8
;;                                     v156 = iconst.i32 24
;; @0025                               v24 = uadd_overflow_trap v23, v156, user18  ; v156 = 24
;; @0025                               v139 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v26 = load.i64 notrap aligned v139+40
;; @0025                               v25 = uextend.i64 v24
;; @0025                               v27 = icmp ule v25, v26
;; @0025                               brif v27, block2, block3
;;
;;                                 block2:
;;                                     v170 = iconst.i32 -1476394984
;; @0025                               v31 = load.i64 notrap aligned readonly can_move v139+32
;;                                     v268 = band.i32 v21, v169  ; v169 = -8
;;                                     v269 = uextend.i64 v268
;; @0025                               v33 = iadd v31, v269
;; @0025                               store user2 region1 v170, v33  ; v170 = -1476394984
;; @0025                               v37 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v38 = load.i32 notrap aligned readonly can_move v37
;; @0025                               store user2 region1 v38, v33+4
;; @0025                               store.i32 user2 region1 v24, v17
;; @0025                               v6 = iconst.i32 3
;; @0025                               v39 = iconst.i64 8
;; @0025                               v40 = iadd v33, v39  ; v39 = 8
;; @0025                               store user2 region1 v6, v40  ; v6 = 3
;; @0025                               trapz v268, user16
;;                                     v270 = iconst.i32 24
;; @0025                               v59 = uadd_overflow_trap v268, v270, user2  ; v270 = 24
;;                                     v118 = load.i32 notrap v145
;; @0025                               v60 = uextend.i64 v59
;; @0025                               v62 = iadd v31, v60
;;                                     v147 = iconst.i64 12
;; @0025                               v65 = isub v62, v147  ; v147 = 12
;; @0025                               store user2 little region1 v118, v65
;; @0025                               v72 = load.i32 user2 readonly region1 v40
;; @0025                               v66 = iconst.i32 1
;;                                     v208 = icmp ugt v72, v66  ; v66 = 1
;; @0025                               trapz v208, user17
;; @0025                               v75 = uextend.i64 v72
;;                                     v148 = iconst.i64 2
;;                                     v210 = ishl v75, v148  ; v148 = 2
;;                                     v141 = iconst.i64 32
;; @0025                               v77 = ushr v210, v141  ; v141 = 32
;; @0025                               trapnz v77, user2
;;                                     v187 = iconst.i32 2
;;                                     v217 = ishl v72, v187  ; v187 = 2
;; @0025                               v7 = iconst.i32 12
;; @0025                               v80 = uadd_overflow_trap v217, v7, user2  ; v7 = 12
;; @0025                               v84 = uadd_overflow_trap v268, v80, user2
;;                                     v117 = load.i32 notrap v144
;; @0025                               v85 = uextend.i64 v84
;; @0025                               v87 = iadd v31, v85
;;                                     v230 = iconst.i32 16
;; @0025                               v88 = isub v80, v230  ; v230 = 16
;; @0025                               v89 = uextend.i64 v88
;; @0025                               v90 = isub v87, v89
;; @0025                               store user2 little region1 v117, v90
;; @0025                               v97 = load.i32 user2 readonly region1 v40
;;                                     v236 = icmp ugt v97, v187  ; v187 = 2
;; @0025                               trapz v236, user17
;; @0025                               v100 = uextend.i64 v97
;;                                     v238 = ishl v100, v148  ; v148 = 2
;; @0025                               v102 = ushr v238, v141  ; v141 = 32
;; @0025                               trapnz v102, user2
;;                                     v245 = ishl v97, v187  ; v187 = 2
;; @0025                               v105 = uadd_overflow_trap v245, v7, user2  ; v7 = 12
;; @0025                               v109 = uadd_overflow_trap v268, v105, user2
;;                                     v116 = load.i32 notrap v143
;; @0025                               v110 = uextend.i64 v109
;; @0025                               v112 = iadd v31, v110
;;                                     v262 = iconst.i32 20
;; @0025                               v113 = isub v105, v262  ; v262 = 20
;; @0025                               v114 = uextend.i64 v113
;; @0025                               v115 = isub v112, v114
;; @0025                               store user2 little region1 v116, v115
;; @0029                               jump block1
;;
;;                                 block3 cold:
;; @0025                               v29 = isub.i64 v25, v26
;; @0025                               v30 = call fn0(v0, v29), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]
;; @0025                               jump block2
;;
;;                                 block1:
;;                                     v271 = band.i32 v21, v169  ; v169 = -8
;; @0029                               return v271
;; }
