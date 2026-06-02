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
;; @0025                               v17 = load.i64 notrap aligned readonly region0 v0+32
;; @0025                               v18 = load.i32 user2 region1 v17
;;                                     v154 = iconst.i32 7
;; @0025                               v21 = uadd_overflow_trap v18, v154, user18  ; v154 = 7
;;                                     v160 = iconst.i32 -8
;; @0025                               v23 = band v21, v160  ; v160 = -8
;;                                     v147 = iconst.i32 40
;; @0025                               v24 = uadd_overflow_trap v23, v147, user18  ; v147 = 40
;; @0025                               v133 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v26 = load.i64 notrap aligned v133+40
;; @0025                               v25 = uextend.i64 v24
;; @0025                               v27 = icmp ule v25, v26
;; @0025                               brif v27, block2, block3
;;
;;                                 block2:
;;                                     v161 = iconst.i32 -1476394968
;; @0025                               v31 = load.i64 notrap aligned readonly can_move v133+32
;;                                     v256 = band.i32 v21, v160  ; v160 = -8
;;                                     v257 = uextend.i64 v256
;; @0025                               v33 = iadd v31, v257
;; @0025                               store user2 region1 v161, v33  ; v161 = -1476394968
;; @0025                               v37 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v38 = load.i32 notrap aligned readonly can_move v37
;; @0025                               store user2 region1 v38, v33+4
;; @0025                               store.i32 user2 region1 v24, v17
;; @0025                               v6 = iconst.i32 3
;;                                     v136 = iconst.i64 8
;; @0025                               v40 = iadd v33, v136  ; v136 = 8
;; @0025                               store user2 region1 v6, v40  ; v6 = 3
;; @0025                               trapz v256, user16
;;                                     v258 = iconst.i32 40
;; @0025                               v59 = uadd_overflow_trap v256, v258, user2  ; v258 = 40
;; @0025                               v60 = uextend.i64 v59
;; @0025                               v62 = iadd v31, v60
;;                                     v138 = iconst.i64 24
;; @0025                               v65 = isub v62, v138  ; v138 = 24
;; @0025                               store.i64 user2 little region1 v2, v65
;; @0025                               v72 = load.i32 user2 readonly region1 v40
;; @0025                               v66 = iconst.i32 1
;;                                     v197 = icmp ugt v72, v66  ; v66 = 1
;; @0025                               trapz v197, user17
;; @0025                               v75 = uextend.i64 v72
;;                                     v137 = iconst.i64 3
;;                                     v199 = ishl v75, v137  ; v137 = 3
;;                                     v135 = iconst.i64 32
;; @0025                               v77 = ushr v199, v135  ; v135 = 32
;; @0025                               trapnz v77, user2
;;                                     v206 = ishl v72, v6  ; v6 = 3
;; @0025                               v7 = iconst.i32 16
;; @0025                               v80 = uadd_overflow_trap v206, v7, user2  ; v7 = 16
;; @0025                               v84 = uadd_overflow_trap v256, v80, user2
;; @0025                               v85 = uextend.i64 v84
;; @0025                               v87 = iadd v31, v85
;;                                     v146 = iconst.i32 24
;; @0025                               v88 = isub v80, v146  ; v146 = 24
;; @0025                               v89 = uextend.i64 v88
;; @0025                               v90 = isub v87, v89
;; @0025                               store.i64 user2 little region1 v3, v90
;; @0025                               v97 = load.i32 user2 readonly region1 v40
;; @0025                               v91 = iconst.i32 2
;;                                     v224 = icmp ugt v97, v91  ; v91 = 2
;; @0025                               trapz v224, user17
;; @0025                               v100 = uextend.i64 v97
;;                                     v226 = ishl v100, v137  ; v137 = 3
;; @0025                               v102 = ushr v226, v135  ; v135 = 32
;; @0025                               trapnz v102, user2
;;                                     v233 = ishl v97, v6  ; v6 = 3
;; @0025                               v105 = uadd_overflow_trap v233, v7, user2  ; v7 = 16
;; @0025                               v109 = uadd_overflow_trap v256, v105, user2
;; @0025                               v110 = uextend.i64 v109
;; @0025                               v112 = iadd v31, v110
;;                                     v250 = iconst.i32 32
;; @0025                               v113 = isub v105, v250  ; v250 = 32
;; @0025                               v114 = uextend.i64 v113
;; @0025                               v115 = isub v112, v114
;; @0025                               store.i64 user2 little region1 v4, v115
;; @0029                               jump block1
;;
;;                                 block3 cold:
;; @0025                               v29 = isub.i64 v25, v26
;; @0025                               v30 = call fn0(v0, v29)
;; @0025                               jump block2
;;
;;                                 block1:
;;                                     v259 = band.i32 v21, v160  ; v160 = -8
;; @0029                               return v259
;; }
