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
;; @0025                               v15 = iconst.i32 -1476395008
;; @0025                               v17 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v18 = load.i32 notrap aligned readonly can_move v17
;;                                     v129 = iconst.i32 56
;; @0025                               v19 = iconst.i32 8
;; @0025                               v20 = call fn0(v0, v15, v18, v129, v19)  ; v15 = -1476395008, v129 = 56, v19 = 8
;; @0025                               v6 = iconst.i32 3
;; @0025                               v116 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v21 = load.i64 notrap aligned readonly can_move v116+32
;; @0025                               v22 = uextend.i64 v20
;; @0025                               v23 = iadd v21, v22
;;                                     v120 = iconst.i64 24
;; @0025                               v25 = iadd v23, v120  ; v120 = 24
;; @0025                               store user2 region0 v6, v25  ; v6 = 3
;; @0025                               trapz v20, user16
;; @0025                               v44 = uadd_overflow_trap v20, v129, user2  ; v129 = 56
;; @0025                               v45 = uextend.i64 v44
;; @0025                               v47 = iadd v21, v45
;; @0025                               v50 = isub v47, v120  ; v120 = 24
;; @0025                               store user2 little region0 v2, v50
;; @0025                               v57 = load.i32 user2 readonly region0 v25
;; @0025                               v51 = iconst.i32 1
;;                                     v160 = icmp ugt v57, v51  ; v51 = 1
;; @0025                               trapz v160, user17
;; @0025                               v60 = uextend.i64 v57
;;                                     v119 = iconst.i64 3
;;                                     v162 = ishl v60, v119  ; v119 = 3
;;                                     v118 = iconst.i64 32
;; @0025                               v62 = ushr v162, v118  ; v118 = 32
;; @0025                               trapnz v62, user2
;;                                     v169 = ishl v57, v6  ; v6 = 3
;; @0025                               v7 = iconst.i32 32
;; @0025                               v65 = uadd_overflow_trap v169, v7, user2  ; v7 = 32
;; @0025                               v69 = uadd_overflow_trap v20, v65, user2
;; @0025                               v70 = uextend.i64 v69
;; @0025                               v72 = iadd v21, v70
;;                                     v182 = iconst.i32 40
;; @0025                               v73 = isub v65, v182  ; v182 = 40
;; @0025                               v74 = uextend.i64 v73
;; @0025                               v75 = isub v72, v74
;; @0025                               store user2 little region0 v3, v75
;; @0025                               v82 = load.i32 user2 readonly region0 v25
;; @0025                               v76 = iconst.i32 2
;;                                     v188 = icmp ugt v82, v76  ; v76 = 2
;; @0025                               trapz v188, user17
;; @0025                               v85 = uextend.i64 v82
;;                                     v190 = ishl v85, v119  ; v119 = 3
;; @0025                               v87 = ushr v190, v118  ; v118 = 32
;; @0025                               trapnz v87, user2
;;                                     v197 = ishl v82, v6  ; v6 = 3
;; @0025                               v90 = uadd_overflow_trap v197, v7, user2  ; v7 = 32
;; @0025                               v94 = uadd_overflow_trap v20, v90, user2
;; @0025                               v95 = uextend.i64 v94
;; @0025                               v97 = iadd v21, v95
;;                                     v215 = iconst.i32 48
;; @0025                               v98 = isub v90, v215  ; v215 = 48
;; @0025                               v99 = uextend.i64 v98
;; @0025                               v100 = isub v97, v99
;; @0025                               store user2 little region0 v4, v100
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v20
;; }
