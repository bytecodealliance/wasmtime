;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $ty (array (mut i64)))

  (func (param i64 i64 i64) (result (ref $ty))
    (array.new_fixed $ty 3 (local.get 0) (local.get 1) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, i64, i64, i64) -> i32 tail {
;;     region0 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;; @0025                               v14 = iconst.i32 -1476395008
;; @0025                               v16 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v17 = load.i32 notrap aligned readonly can_move v16
;;                                     v129 = iconst.i32 56
;; @0025                               v18 = iconst.i32 8
;; @0025                               v19 = call fn0(v0, v14, v17, v129, v18)  ; v14 = -1476395008, v129 = 56, v18 = 8
;; @0025                               v6 = iconst.i32 3
;; @0025                               v115 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v20 = load.i64 notrap aligned readonly can_move v115+32
;; @0025                               v21 = uextend.i64 v19
;; @0025                               v22 = iadd v20, v21
;;                                     v120 = iconst.i64 24
;; @0025                               v24 = iadd v22, v120  ; v120 = 24
;; @0025                               store user2 region0 v6, v24  ; v6 = 3
;; @0025                               trapz v19, user16
;; @0025                               v43 = uadd_overflow_trap v19, v129, user2  ; v129 = 56
;; @0025                               v44 = uextend.i64 v43
;; @0025                               v46 = iadd v20, v44
;; @0025                               v49 = isub v46, v120  ; v120 = 24
;; @0025                               store user2 little region0 v2, v49
;; @0025                               v56 = load.i32 user2 readonly region0 v24
;; @0025                               v50 = iconst.i32 1
;;                                     v160 = icmp ugt v56, v50  ; v50 = 1
;; @0025                               trapz v160, user17
;; @0025                               v59 = uextend.i64 v56
;;                                     v119 = iconst.i64 3
;;                                     v162 = ishl v59, v119  ; v119 = 3
;;                                     v117 = iconst.i64 32
;; @0025                               v61 = ushr v162, v117  ; v117 = 32
;; @0025                               trapnz v61, user2
;;                                     v169 = ishl v56, v6  ; v6 = 3
;; @0025                               v7 = iconst.i32 32
;; @0025                               v64 = uadd_overflow_trap v169, v7, user2  ; v7 = 32
;; @0025                               v68 = uadd_overflow_trap v19, v64, user2
;; @0025                               v69 = uextend.i64 v68
;; @0025                               v71 = iadd v20, v69
;;                                     v182 = iconst.i32 40
;; @0025                               v72 = isub v64, v182  ; v182 = 40
;; @0025                               v73 = uextend.i64 v72
;; @0025                               v74 = isub v71, v73
;; @0025                               store user2 little region0 v3, v74
;; @0025                               v81 = load.i32 user2 readonly region0 v24
;; @0025                               v75 = iconst.i32 2
;;                                     v188 = icmp ugt v81, v75  ; v75 = 2
;; @0025                               trapz v188, user17
;; @0025                               v84 = uextend.i64 v81
;;                                     v190 = ishl v84, v119  ; v119 = 3
;; @0025                               v86 = ushr v190, v117  ; v117 = 32
;; @0025                               trapnz v86, user2
;;                                     v197 = ishl v81, v6  ; v6 = 3
;; @0025                               v89 = uadd_overflow_trap v197, v7, user2  ; v7 = 32
;; @0025                               v93 = uadd_overflow_trap v19, v89, user2
;; @0025                               v94 = uextend.i64 v93
;; @0025                               v96 = iadd v20, v94
;;                                     v215 = iconst.i32 48
;; @0025                               v97 = isub v89, v215  ; v215 = 48
;; @0025                               v98 = uextend.i64 v97
;; @0025                               v99 = isub v96, v98
;; @0025                               store user2 little region0 v4, v99
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v19
;; }
