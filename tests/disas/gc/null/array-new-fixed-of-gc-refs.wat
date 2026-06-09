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
;;                                     v125 = stack_addr.i64 ss2
;;                                     store notrap v2, v125
;;                                     v126 = stack_addr.i64 ss1
;;                                     store notrap v3, v126
;;                                     v127 = stack_addr.i64 ss0
;;                                     store notrap v4, v127
;; @0025                               v18 = load.i64 notrap aligned readonly region0 v0+32
;; @0025                               v19 = load.i32 user2 region1 v18
;;                                     v159 = iconst.i32 7
;; @0025                               v22 = uadd_overflow_trap v19, v159, user18  ; v159 = 7
;;                                     v165 = iconst.i32 -8
;; @0025                               v24 = band v22, v165  ; v165 = -8
;;                                     v152 = iconst.i32 24
;; @0025                               v25 = uadd_overflow_trap v24, v152, user18  ; v152 = 24
;; @0025                               v140 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v27 = load.i64 notrap aligned v140+40
;; @0025                               v26 = uextend.i64 v25
;; @0025                               v28 = icmp ule v26, v27
;; @0025                               brif v28, block2, block3
;;
;;                                 block2:
;;                                     v166 = iconst.i32 -1476394984
;; @0025                               v32 = load.i64 notrap aligned readonly can_move v140+32
;;                                     v264 = band.i32 v22, v165  ; v165 = -8
;;                                     v265 = uextend.i64 v264
;; @0025                               v34 = iadd v32, v265
;; @0025                               store user2 region1 v166, v34  ; v166 = -1476394984
;; @0025                               v37 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v38 = load.i32 notrap aligned readonly can_move v37
;; @0025                               store user2 region1 v38, v34+4
;; @0025                               store.i32 user2 region1 v25, v18
;; @0025                               v6 = iconst.i32 3
;; @0025                               v39 = iconst.i64 8
;; @0025                               v40 = iadd v34, v39  ; v39 = 8
;; @0025                               store user2 region1 v6, v40  ; v6 = 3
;; @0025                               trapz v264, user16
;;                                     v266 = iconst.i32 24
;; @0025                               v60 = uadd_overflow_trap v264, v266, user2  ; v266 = 24
;;                                     v124 = load.i32 notrap v125
;; @0025                               v61 = uextend.i64 v60
;; @0025                               v63 = iadd v32, v61
;;                                     v143 = iconst.i64 12
;; @0025                               v66 = isub v63, v143  ; v143 = 12
;; @0025                               store user2 little region1 v124, v66
;; @0025                               v73 = load.i32 user2 readonly region1 v40
;; @0025                               v67 = iconst.i32 1
;;                                     v204 = icmp ugt v73, v67  ; v67 = 1
;; @0025                               trapz v204, user17
;; @0025                               v76 = uextend.i64 v73
;;                                     v144 = iconst.i64 2
;;                                     v206 = ishl v76, v144  ; v144 = 2
;; @0025                               v11 = iconst.i64 32
;; @0025                               v79 = ushr v206, v11  ; v11 = 32
;; @0025                               trapnz v79, user2
;;                                     v183 = iconst.i32 2
;;                                     v213 = ishl v73, v183  ; v183 = 2
;; @0025                               v7 = iconst.i32 12
;; @0025                               v82 = uadd_overflow_trap v213, v7, user2  ; v7 = 12
;; @0025                               v86 = uadd_overflow_trap v264, v82, user2
;;                                     v122 = load.i32 notrap v126
;; @0025                               v87 = uextend.i64 v86
;; @0025                               v89 = iadd v32, v87
;;                                     v226 = iconst.i32 16
;; @0025                               v90 = isub v82, v226  ; v226 = 16
;; @0025                               v91 = uextend.i64 v90
;; @0025                               v92 = isub v89, v91
;; @0025                               store user2 little region1 v122, v92
;; @0025                               v99 = load.i32 user2 readonly region1 v40
;;                                     v232 = icmp ugt v99, v183  ; v183 = 2
;; @0025                               trapz v232, user17
;; @0025                               v102 = uextend.i64 v99
;;                                     v234 = ishl v102, v144  ; v144 = 2
;; @0025                               v105 = ushr v234, v11  ; v11 = 32
;; @0025                               trapnz v105, user2
;;                                     v241 = ishl v99, v183  ; v183 = 2
;; @0025                               v108 = uadd_overflow_trap v241, v7, user2  ; v7 = 12
;; @0025                               v112 = uadd_overflow_trap v264, v108, user2
;;                                     v120 = load.i32 notrap v127
;; @0025                               v113 = uextend.i64 v112
;; @0025                               v115 = iadd v32, v113
;;                                     v258 = iconst.i32 20
;; @0025                               v116 = isub v108, v258  ; v258 = 20
;; @0025                               v117 = uextend.i64 v116
;; @0025                               v118 = isub v115, v117
;; @0025                               store user2 little region1 v120, v118
;; @0029                               jump block1
;;
;;                                 block3 cold:
;; @0025                               v29 = isub.i64 v26, v27
;; @0025                               v30 = call fn0(v0, v29), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]
;; @0025                               jump block2
;;
;;                                 block1:
;;                                     v267 = band.i32 v22, v165  ; v165 = -8
;; @0029                               return v267
;; }
