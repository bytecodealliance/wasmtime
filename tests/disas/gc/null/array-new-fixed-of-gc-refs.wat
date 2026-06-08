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
;;                                     v124 = stack_addr.i64 ss2
;;                                     store notrap v2, v124
;;                                     v125 = stack_addr.i64 ss1
;;                                     store notrap v3, v125
;;                                     v126 = stack_addr.i64 ss0
;;                                     store notrap v4, v126
;; @0025                               v18 = load.i64 notrap aligned readonly region0 v0+32
;; @0025                               v19 = load.i32 user2 region1 v18
;;                                     v160 = iconst.i32 7
;; @0025                               v22 = uadd_overflow_trap v19, v160, user18  ; v160 = 7
;;                                     v166 = iconst.i32 -8
;; @0025                               v24 = band v22, v166  ; v166 = -8
;;                                     v153 = iconst.i32 24
;; @0025                               v25 = uadd_overflow_trap v24, v153, user18  ; v153 = 24
;; @0025                               v141 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v27 = load.i64 notrap aligned v141+40
;; @0025                               v26 = uextend.i64 v25
;; @0025                               v28 = icmp ule v26, v27
;; @0025                               brif v28, block2, block3
;;
;;                                 block2:
;;                                     v167 = iconst.i32 -1476394984
;; @0025                               v31 = load.i64 notrap aligned readonly can_move v141+32
;;                                     v265 = band.i32 v22, v166  ; v166 = -8
;;                                     v266 = uextend.i64 v265
;; @0025                               v33 = iadd v31, v266
;; @0025                               store user2 region1 v167, v33  ; v167 = -1476394984
;; @0025                               v36 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v37 = load.i32 notrap aligned readonly can_move v36
;; @0025                               store user2 region1 v37, v33+4
;; @0025                               store.i32 user2 region1 v25, v18
;; @0025                               v6 = iconst.i32 3
;; @0025                               v38 = iconst.i64 8
;; @0025                               v39 = iadd v33, v38  ; v38 = 8
;; @0025                               store user2 region1 v6, v39  ; v6 = 3
;; @0025                               trapz v265, user16
;;                                     v267 = iconst.i32 24
;; @0025                               v59 = uadd_overflow_trap v265, v267, user2  ; v267 = 24
;;                                     v123 = load.i32 notrap v124
;; @0025                               v60 = uextend.i64 v59
;; @0025                               v62 = iadd v31, v60
;;                                     v144 = iconst.i64 12
;; @0025                               v65 = isub v62, v144  ; v144 = 12
;; @0025                               store user2 little region1 v123, v65
;; @0025                               v72 = load.i32 user2 readonly region1 v39
;; @0025                               v66 = iconst.i32 1
;;                                     v205 = icmp ugt v72, v66  ; v66 = 1
;; @0025                               trapz v205, user17
;; @0025                               v75 = uextend.i64 v72
;;                                     v145 = iconst.i64 2
;;                                     v207 = ishl v75, v145  ; v145 = 2
;; @0025                               v11 = iconst.i64 32
;; @0025                               v78 = ushr v207, v11  ; v11 = 32
;; @0025                               trapnz v78, user2
;;                                     v184 = iconst.i32 2
;;                                     v214 = ishl v72, v184  ; v184 = 2
;; @0025                               v7 = iconst.i32 12
;; @0025                               v81 = uadd_overflow_trap v214, v7, user2  ; v7 = 12
;; @0025                               v85 = uadd_overflow_trap v265, v81, user2
;;                                     v121 = load.i32 notrap v125
;; @0025                               v86 = uextend.i64 v85
;; @0025                               v88 = iadd v31, v86
;;                                     v227 = iconst.i32 16
;; @0025                               v89 = isub v81, v227  ; v227 = 16
;; @0025                               v90 = uextend.i64 v89
;; @0025                               v91 = isub v88, v90
;; @0025                               store user2 little region1 v121, v91
;; @0025                               v98 = load.i32 user2 readonly region1 v39
;;                                     v233 = icmp ugt v98, v184  ; v184 = 2
;; @0025                               trapz v233, user17
;; @0025                               v101 = uextend.i64 v98
;;                                     v235 = ishl v101, v145  ; v145 = 2
;; @0025                               v104 = ushr v235, v11  ; v11 = 32
;; @0025                               trapnz v104, user2
;;                                     v242 = ishl v98, v184  ; v184 = 2
;; @0025                               v107 = uadd_overflow_trap v242, v7, user2  ; v7 = 12
;; @0025                               v111 = uadd_overflow_trap v265, v107, user2
;;                                     v119 = load.i32 notrap v126
;; @0025                               v112 = uextend.i64 v111
;; @0025                               v114 = iadd v31, v112
;;                                     v259 = iconst.i32 20
;; @0025                               v115 = isub v107, v259  ; v259 = 20
;; @0025                               v116 = uextend.i64 v115
;; @0025                               v117 = isub v114, v116
;; @0025                               store user2 little region1 v119, v117
;; @0029                               jump block1
;;
;;                                 block3 cold:
;; @0025                               v29 = isub.i64 v26, v27
;; @0025                               v30 = call fn0(v0, v29), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]
;; @0025                               jump block2
;;
;;                                 block1:
;;                                     v268 = band.i32 v22, v166  ; v166 = -8
;; @0029                               return v268
;; }
