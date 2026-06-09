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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 32 "VMContext+0x20"
;;     region2 = 2147483648 "GcHeap"
;;     region3 = 40 "VMContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     fn0 = colocated u805306368:23 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;; @0025                               v18 = load.i64 notrap aligned readonly region1 v0+32
;; @0025                               v19 = load.i32 user2 region2 v18
;;                                     v149 = iconst.i32 7
;; @0025                               v22 = uadd_overflow_trap v19, v149, user18  ; v149 = 7
;;                                     v155 = iconst.i32 -8
;; @0025                               v24 = band v22, v155  ; v155 = -8
;;                                     v142 = iconst.i32 40
;; @0025                               v25 = uadd_overflow_trap v24, v142, user18  ; v142 = 40
;; @0025                               v27 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0025                               v28 = load.i64 notrap aligned v27+40
;; @0025                               v26 = uextend.i64 v25
;; @0025                               v29 = icmp ule v26, v28
;; @0025                               brif v29, block2, block3
;;
;;                                 block2:
;;                                     v156 = iconst.i32 -1476394968
;; @0025                               v33 = load.i64 notrap aligned readonly can_move v27+32
;;                                     v251 = band.i32 v22, v155  ; v155 = -8
;;                                     v252 = uextend.i64 v251
;; @0025                               v35 = iadd v33, v252
;; @0025                               store user2 region2 v156, v35  ; v156 = -1476394968
;; @0025                               v38 = load.i64 notrap aligned readonly can_move region3 v0+40
;; @0025                               v39 = load.i32 notrap aligned readonly can_move v38
;; @0025                               store user2 region2 v39, v35+4
;; @0025                               store.i32 user2 region2 v25, v18
;; @0025                               v6 = iconst.i32 3
;; @0025                               v9 = iconst.i64 8
;; @0025                               v41 = iadd v35, v9  ; v9 = 8
;; @0025                               store user2 region2 v6, v41  ; v6 = 3
;; @0025                               trapz v251, user16
;;                                     v253 = iconst.i32 40
;; @0025                               v61 = uadd_overflow_trap v251, v253, user2  ; v253 = 40
;; @0025                               v62 = uextend.i64 v61
;; @0025                               v64 = iadd v33, v62
;;                                     v133 = iconst.i64 24
;; @0025                               v67 = isub v64, v133  ; v133 = 24
;; @0025                               store.i64 user2 little region2 v2, v67
;; @0025                               v74 = load.i32 user2 readonly region2 v41
;; @0025                               v68 = iconst.i32 1
;;                                     v192 = icmp ugt v74, v68  ; v68 = 1
;; @0025                               trapz v192, user17
;; @0025                               v77 = uextend.i64 v74
;;                                     v132 = iconst.i64 3
;;                                     v194 = ishl v77, v132  ; v132 = 3
;; @0025                               v11 = iconst.i64 32
;; @0025                               v80 = ushr v194, v11  ; v11 = 32
;; @0025                               trapnz v80, user2
;;                                     v201 = ishl v74, v6  ; v6 = 3
;; @0025                               v7 = iconst.i32 16
;; @0025                               v83 = uadd_overflow_trap v201, v7, user2  ; v7 = 16
;; @0025                               v87 = uadd_overflow_trap v251, v83, user2
;; @0025                               v88 = uextend.i64 v87
;; @0025                               v90 = iadd v33, v88
;;                                     v141 = iconst.i32 24
;; @0025                               v91 = isub v83, v141  ; v141 = 24
;; @0025                               v92 = uextend.i64 v91
;; @0025                               v93 = isub v90, v92
;; @0025                               store.i64 user2 little region2 v3, v93
;; @0025                               v100 = load.i32 user2 readonly region2 v41
;; @0025                               v94 = iconst.i32 2
;;                                     v219 = icmp ugt v100, v94  ; v94 = 2
;; @0025                               trapz v219, user17
;; @0025                               v103 = uextend.i64 v100
;;                                     v221 = ishl v103, v132  ; v132 = 3
;; @0025                               v106 = ushr v221, v11  ; v11 = 32
;; @0025                               trapnz v106, user2
;;                                     v228 = ishl v100, v6  ; v6 = 3
;; @0025                               v109 = uadd_overflow_trap v228, v7, user2  ; v7 = 16
;; @0025                               v113 = uadd_overflow_trap v251, v109, user2
;; @0025                               v114 = uextend.i64 v113
;; @0025                               v116 = iadd v33, v114
;;                                     v245 = iconst.i32 32
;; @0025                               v117 = isub v109, v245  ; v245 = 32
;; @0025                               v118 = uextend.i64 v117
;; @0025                               v119 = isub v116, v118
;; @0025                               store.i64 user2 little region2 v4, v119
;; @0029                               jump block1
;;
;;                                 block3 cold:
;; @0025                               v30 = isub.i64 v26, v28
;; @0025                               v31 = call fn0(v0, v30)
;; @0025                               jump block2
;;
;;                                 block1:
;;                                     v254 = band.i32 v22, v155  ; v155 = -8
;; @0029                               return v254
;; }
