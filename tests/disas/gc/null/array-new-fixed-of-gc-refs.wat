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
;; @0025                               v18 = load.i64 notrap aligned readonly region0 v0+32
;; @0025                               v19 = load.i32 user2 region1 v18
;;                                     v163 = iconst.i32 7
;; @0025                               v22 = uadd_overflow_trap v19, v163, user18  ; v163 = 7
;;                                     v169 = iconst.i32 -8
;; @0025                               v24 = band v22, v169  ; v169 = -8
;;                                     v156 = iconst.i32 24
;; @0025                               v25 = uadd_overflow_trap v24, v156, user18  ; v156 = 24
;; @0025                               v140 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v27 = load.i64 notrap aligned v140+40
;; @0025                               v26 = uextend.i64 v25
;; @0025                               v28 = icmp ule v26, v27
;; @0025                               brif v28, block2, block3
;;
;;                                 block2:
;;                                     v170 = iconst.i32 -1476394984
;; @0025                               v32 = load.i64 notrap aligned readonly can_move v140+32
;;                                     v268 = band.i32 v22, v169  ; v169 = -8
;;                                     v269 = uextend.i64 v268
;; @0025                               v34 = iadd v32, v269
;; @0025                               store user2 region1 v170, v34  ; v170 = -1476394984
;; @0025                               v38 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v39 = load.i32 notrap aligned readonly can_move v38
;; @0025                               store user2 region1 v39, v34+4
;; @0025                               store.i32 user2 region1 v25, v18
;; @0025                               v6 = iconst.i32 3
;; @0025                               v40 = iconst.i64 8
;; @0025                               v41 = iadd v34, v40  ; v40 = 8
;; @0025                               store user2 region1 v6, v41  ; v6 = 3
;; @0025                               trapz v268, user16
;;                                     v270 = iconst.i32 24
;; @0025                               v60 = uadd_overflow_trap v268, v270, user2  ; v270 = 24
;;                                     v119 = load.i32 notrap v145
;; @0025                               v61 = uextend.i64 v60
;; @0025                               v63 = iadd v32, v61
;;                                     v147 = iconst.i64 12
;; @0025                               v66 = isub v63, v147  ; v147 = 12
;; @0025                               store user2 little region1 v119, v66
;; @0025                               v73 = load.i32 user2 readonly region1 v41
;; @0025                               v67 = iconst.i32 1
;;                                     v208 = icmp ugt v73, v67  ; v67 = 1
;; @0025                               trapz v208, user17
;; @0025                               v76 = uextend.i64 v73
;;                                     v148 = iconst.i64 2
;;                                     v210 = ishl v76, v148  ; v148 = 2
;;                                     v142 = iconst.i64 32
;; @0025                               v78 = ushr v210, v142  ; v142 = 32
;; @0025                               trapnz v78, user2
;;                                     v187 = iconst.i32 2
;;                                     v217 = ishl v73, v187  ; v187 = 2
;; @0025                               v7 = iconst.i32 12
;; @0025                               v81 = uadd_overflow_trap v217, v7, user2  ; v7 = 12
;; @0025                               v85 = uadd_overflow_trap v268, v81, user2
;;                                     v118 = load.i32 notrap v144
;; @0025                               v86 = uextend.i64 v85
;; @0025                               v88 = iadd v32, v86
;;                                     v230 = iconst.i32 16
;; @0025                               v89 = isub v81, v230  ; v230 = 16
;; @0025                               v90 = uextend.i64 v89
;; @0025                               v91 = isub v88, v90
;; @0025                               store user2 little region1 v118, v91
;; @0025                               v98 = load.i32 user2 readonly region1 v41
;;                                     v236 = icmp ugt v98, v187  ; v187 = 2
;; @0025                               trapz v236, user17
;; @0025                               v101 = uextend.i64 v98
;;                                     v238 = ishl v101, v148  ; v148 = 2
;; @0025                               v103 = ushr v238, v142  ; v142 = 32
;; @0025                               trapnz v103, user2
;;                                     v245 = ishl v98, v187  ; v187 = 2
;; @0025                               v106 = uadd_overflow_trap v245, v7, user2  ; v7 = 12
;; @0025                               v110 = uadd_overflow_trap v268, v106, user2
;;                                     v117 = load.i32 notrap v143
;; @0025                               v111 = uextend.i64 v110
;; @0025                               v113 = iadd v32, v111
;;                                     v262 = iconst.i32 20
;; @0025                               v114 = isub v106, v262  ; v262 = 20
;; @0025                               v115 = uextend.i64 v114
;; @0025                               v116 = isub v113, v115
;; @0025                               store user2 little region1 v117, v116
;; @0029                               jump block1
;;
;;                                 block3 cold:
;; @0025                               v30 = isub.i64 v26, v27
;; @0025                               v31 = call fn0(v0, v30), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]
;; @0025                               jump block2
;;
;;                                 block1:
;;                                     v271 = band.i32 v22, v169  ; v169 = -8
;; @0029                               return v271
;; }
