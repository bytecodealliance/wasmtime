;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
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
;;     gv5 = load.i64 notrap aligned gv4+40
;;     gv6 = load.i64 notrap aligned readonly can_move gv4+32
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     fn0 = colocated u805306368:23 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;; @0025                               v18 = load.i64 notrap aligned readonly region0 v0+32
;; @0025                               v19 = load.i32 user2 region1 v18
;;                                     v154 = iconst.i32 7
;; @0025                               v22 = uadd_overflow_trap v19, v154, user18  ; v154 = 7
;;                                     v160 = iconst.i32 -8
;; @0025                               v24 = band v22, v160  ; v160 = -8
;;                                     v147 = iconst.i32 40
;; @0025                               v25 = uadd_overflow_trap v24, v147, user18  ; v147 = 40
;; @0025                               v134 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v27 = load.i64 notrap aligned v134+40
;; @0025                               v26 = uextend.i64 v25
;; @0025                               v28 = icmp ule v26, v27
;; @0025                               brif v28, block2, block3
;;
;;                                 block2:
;;                                     v161 = iconst.i32 -1476394968
;; @0025                               v32 = load.i64 notrap aligned readonly can_move v134+32
;;                                     v256 = band.i32 v22, v160  ; v160 = -8
;;                                     v257 = uextend.i64 v256
;; @0025                               v34 = iadd v32, v257
;; @0025                               store user2 region1 v161, v34  ; v161 = -1476394968
;; @0025                               v38 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v39 = load.i32 notrap aligned readonly can_move v38
;; @0025                               store user2 region1 v39, v34+4
;; @0025                               store.i32 user2 region1 v25, v18
;; @0025                               v6 = iconst.i32 3
;; @0025                               v9 = iconst.i64 8
;; @0025                               v41 = iadd v34, v9  ; v9 = 8
;; @0025                               store user2 region1 v6, v41  ; v6 = 3
;; @0025                               trapz v256, user16
;;                                     v258 = iconst.i32 40
;; @0025                               v60 = uadd_overflow_trap v256, v258, user2  ; v258 = 40
;; @0025                               v61 = uextend.i64 v60
;; @0025                               v63 = iadd v32, v61
;;                                     v138 = iconst.i64 24
;; @0025                               v66 = isub v63, v138  ; v138 = 24
;; @0025                               store.i64 user2 little region1 v2, v66
;; @0025                               v73 = load.i32 user2 readonly region1 v41
;; @0025                               v67 = iconst.i32 1
;;                                     v197 = icmp ugt v73, v67  ; v67 = 1
;; @0025                               trapz v197, user17
;; @0025                               v76 = uextend.i64 v73
;;                                     v137 = iconst.i64 3
;;                                     v199 = ishl v76, v137  ; v137 = 3
;;                                     v136 = iconst.i64 32
;; @0025                               v78 = ushr v199, v136  ; v136 = 32
;; @0025                               trapnz v78, user2
;;                                     v206 = ishl v73, v6  ; v6 = 3
;; @0025                               v7 = iconst.i32 16
;; @0025                               v81 = uadd_overflow_trap v206, v7, user2  ; v7 = 16
;; @0025                               v85 = uadd_overflow_trap v256, v81, user2
;; @0025                               v86 = uextend.i64 v85
;; @0025                               v88 = iadd v32, v86
;;                                     v146 = iconst.i32 24
;; @0025                               v89 = isub v81, v146  ; v146 = 24
;; @0025                               v90 = uextend.i64 v89
;; @0025                               v91 = isub v88, v90
;; @0025                               store.i64 user2 little region1 v3, v91
;; @0025                               v98 = load.i32 user2 readonly region1 v41
;; @0025                               v92 = iconst.i32 2
;;                                     v224 = icmp ugt v98, v92  ; v92 = 2
;; @0025                               trapz v224, user17
;; @0025                               v101 = uextend.i64 v98
;;                                     v226 = ishl v101, v137  ; v137 = 3
;; @0025                               v103 = ushr v226, v136  ; v136 = 32
;; @0025                               trapnz v103, user2
;;                                     v233 = ishl v98, v6  ; v6 = 3
;; @0025                               v106 = uadd_overflow_trap v233, v7, user2  ; v7 = 16
;; @0025                               v110 = uadd_overflow_trap v256, v106, user2
;; @0025                               v111 = uextend.i64 v110
;; @0025                               v113 = iadd v32, v111
;;                                     v250 = iconst.i32 32
;; @0025                               v114 = isub v106, v250  ; v250 = 32
;; @0025                               v115 = uextend.i64 v114
;; @0025                               v116 = isub v113, v115
;; @0025                               store.i64 user2 little region1 v4, v116
;; @0029                               jump block1
;;
;;                                 block3 cold:
;; @0025                               v30 = isub.i64 v26, v27
;; @0025                               v31 = call fn0(v0, v30)
;; @0025                               jump block2
;;
;;                                 block1:
;;                                     v259 = band.i32 v22, v160  ; v160 = -8
;; @0029                               return v259
;; }
