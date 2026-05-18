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
;; @0025                               v23 = iadd v22, v120  ; v120 = 24
;; @0025                               store user2 v6, v23  ; v6 = 3
;; @0025                               trapz v19, user16
;; @0025                               v42 = uadd_overflow_trap v19, v129, user2  ; v129 = 56
;; @0025                               v43 = uextend.i64 v42
;; @0025                               v45 = iadd v20, v43
;; @0025                               v48 = isub v45, v120  ; v120 = 24
;; @0025                               store user2 little v2, v48
;; @0025                               v55 = load.i32 user2 readonly v23
;; @0025                               v49 = iconst.i32 1
;;                                     v160 = icmp ugt v55, v49  ; v49 = 1
;; @0025                               trapz v160, user17
;; @0025                               v58 = uextend.i64 v55
;;                                     v119 = iconst.i64 3
;;                                     v162 = ishl v58, v119  ; v119 = 3
;;                                     v117 = iconst.i64 32
;; @0025                               v60 = ushr v162, v117  ; v117 = 32
;; @0025                               trapnz v60, user2
;;                                     v169 = ishl v55, v6  ; v6 = 3
;; @0025                               v7 = iconst.i32 32
;; @0025                               v63 = uadd_overflow_trap v169, v7, user2  ; v7 = 32
;; @0025                               v67 = uadd_overflow_trap v19, v63, user2
;; @0025                               v68 = uextend.i64 v67
;; @0025                               v70 = iadd v20, v68
;;                                     v182 = iconst.i32 40
;; @0025                               v71 = isub v63, v182  ; v182 = 40
;; @0025                               v72 = uextend.i64 v71
;; @0025                               v73 = isub v70, v72
;; @0025                               store user2 little v3, v73
;; @0025                               v80 = load.i32 user2 readonly v23
;; @0025                               v74 = iconst.i32 2
;;                                     v188 = icmp ugt v80, v74  ; v74 = 2
;; @0025                               trapz v188, user17
;; @0025                               v83 = uextend.i64 v80
;;                                     v190 = ishl v83, v119  ; v119 = 3
;; @0025                               v85 = ushr v190, v117  ; v117 = 32
;; @0025                               trapnz v85, user2
;;                                     v197 = ishl v80, v6  ; v6 = 3
;; @0025                               v88 = uadd_overflow_trap v197, v7, user2  ; v7 = 32
;; @0025                               v92 = uadd_overflow_trap v19, v88, user2
;; @0025                               v93 = uextend.i64 v92
;; @0025                               v95 = iadd v20, v93
;;                                     v215 = iconst.i32 48
;; @0025                               v96 = isub v88, v215  ; v215 = 48
;; @0025                               v97 = uextend.i64 v96
;; @0025                               v98 = isub v95, v97
;; @0025                               store user2 little v4, v98
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v19
;; }
