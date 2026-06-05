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
;; @0025                               v19 = load.i64 notrap aligned readonly region0 v0+32
;; @0025                               v20 = load.i32 user2 region1 v19
;;                                     v154 = iconst.i32 7
;; @0025                               v23 = uadd_overflow_trap v20, v154, user18  ; v154 = 7
;;                                     v160 = iconst.i32 -8
;; @0025                               v25 = band v23, v160  ; v160 = -8
;;                                     v147 = iconst.i32 40
;; @0025                               v26 = uadd_overflow_trap v25, v147, user18  ; v147 = 40
;; @0025                               v135 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v28 = load.i64 notrap aligned v135+40
;; @0025                               v27 = uextend.i64 v26
;; @0025                               v29 = icmp ule v27, v28
;; @0025                               brif v29, block2, block3
;;
;;                                 block2:
;;                                     v161 = iconst.i32 -1476394968
;; @0025                               v33 = load.i64 notrap aligned readonly can_move v135+32
;;                                     v256 = band.i32 v23, v160  ; v160 = -8
;;                                     v257 = uextend.i64 v256
;; @0025                               v35 = iadd v33, v257
;; @0025                               store user2 region1 v161, v35  ; v161 = -1476394968
;; @0025                               v39 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v40 = load.i32 notrap aligned readonly can_move v39
;; @0025                               store user2 region1 v40, v35+4
;; @0025                               store.i32 user2 region1 v26, v19
;; @0025                               v6 = iconst.i32 3
;; @0025                               v9 = iconst.i64 8
;; @0025                               v42 = iadd v35, v9  ; v9 = 8
;; @0025                               store user2 region1 v6, v42  ; v6 = 3
;; @0025                               trapz v256, user16
;;                                     v258 = iconst.i32 40
;; @0025                               v62 = uadd_overflow_trap v256, v258, user2  ; v258 = 40
;; @0025                               v63 = uextend.i64 v62
;; @0025                               v65 = iadd v33, v63
;;                                     v138 = iconst.i64 24
;; @0025                               v68 = isub v65, v138  ; v138 = 24
;; @0025                               store.i64 user2 little region1 v2, v68
;; @0025                               v75 = load.i32 user2 readonly region1 v42
;; @0025                               v69 = iconst.i32 1
;;                                     v197 = icmp ugt v75, v69  ; v69 = 1
;; @0025                               trapz v197, user17
;; @0025                               v78 = uextend.i64 v75
;;                                     v137 = iconst.i64 3
;;                                     v199 = ishl v78, v137  ; v137 = 3
;; @0025                               v11 = iconst.i64 32
;; @0025                               v81 = ushr v199, v11  ; v11 = 32
;; @0025                               trapnz v81, user2
;;                                     v206 = ishl v75, v6  ; v6 = 3
;; @0025                               v7 = iconst.i32 16
;; @0025                               v84 = uadd_overflow_trap v206, v7, user2  ; v7 = 16
;; @0025                               v88 = uadd_overflow_trap v256, v84, user2
;; @0025                               v89 = uextend.i64 v88
;; @0025                               v91 = iadd v33, v89
;;                                     v146 = iconst.i32 24
;; @0025                               v92 = isub v84, v146  ; v146 = 24
;; @0025                               v93 = uextend.i64 v92
;; @0025                               v94 = isub v91, v93
;; @0025                               store.i64 user2 little region1 v3, v94
;; @0025                               v101 = load.i32 user2 readonly region1 v42
;; @0025                               v95 = iconst.i32 2
;;                                     v224 = icmp ugt v101, v95  ; v95 = 2
;; @0025                               trapz v224, user17
;; @0025                               v104 = uextend.i64 v101
;;                                     v226 = ishl v104, v137  ; v137 = 3
;; @0025                               v107 = ushr v226, v11  ; v11 = 32
;; @0025                               trapnz v107, user2
;;                                     v233 = ishl v101, v6  ; v6 = 3
;; @0025                               v110 = uadd_overflow_trap v233, v7, user2  ; v7 = 16
;; @0025                               v114 = uadd_overflow_trap v256, v110, user2
;; @0025                               v115 = uextend.i64 v114
;; @0025                               v117 = iadd v33, v115
;;                                     v250 = iconst.i32 32
;; @0025                               v118 = isub v110, v250  ; v250 = 32
;; @0025                               v119 = uextend.i64 v118
;; @0025                               v120 = isub v117, v119
;; @0025                               store.i64 user2 little region1 v4, v120
;; @0029                               jump block1
;;
;;                                 block3 cold:
;; @0025                               v31 = isub.i64 v27, v28
;; @0025                               v32 = call fn0(v0, v31)
;; @0025                               jump block2
;;
;;                                 block1:
;;                                     v259 = band.i32 v23, v160  ; v160 = -8
;; @0029                               return v259
;; }
