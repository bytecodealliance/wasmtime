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
;; @0025                               v16 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v17 = load.i32 notrap aligned readonly can_move v16
;;                                     v127 = iconst.i32 56
;; @0025                               v18 = iconst.i32 8
;; @0025                               v19 = call fn0(v0, v15, v17, v127, v18)  ; v15 = -1476395008, v127 = 56, v18 = 8
;; @0025                               v6 = iconst.i32 3
;; @0025                               v115 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v20 = load.i64 notrap aligned readonly can_move v115+32
;; @0025                               v21 = uextend.i64 v19
;; @0025                               v22 = iadd v20, v21
;;                                     v118 = iconst.i64 24
;; @0025                               v24 = iadd v22, v118  ; v118 = 24
;; @0025                               store user2 region0 v6, v24  ; v6 = 3
;; @0025                               trapz v19, user16
;; @0025                               v44 = uadd_overflow_trap v19, v127, user2  ; v127 = 56
;; @0025                               v45 = uextend.i64 v44
;; @0025                               v47 = iadd v20, v45
;; @0025                               v50 = isub v47, v118  ; v118 = 24
;; @0025                               store user2 little region0 v2, v50
;; @0025                               v57 = load.i32 user2 readonly region0 v24
;; @0025                               v51 = iconst.i32 1
;;                                     v158 = icmp ugt v57, v51  ; v51 = 1
;; @0025                               trapz v158, user17
;; @0025                               v60 = uextend.i64 v57
;;                                     v117 = iconst.i64 3
;;                                     v160 = ishl v60, v117  ; v117 = 3
;; @0025                               v11 = iconst.i64 32
;; @0025                               v63 = ushr v160, v11  ; v11 = 32
;; @0025                               trapnz v63, user2
;;                                     v167 = ishl v57, v6  ; v6 = 3
;; @0025                               v7 = iconst.i32 32
;; @0025                               v66 = uadd_overflow_trap v167, v7, user2  ; v7 = 32
;; @0025                               v70 = uadd_overflow_trap v19, v66, user2
;; @0025                               v71 = uextend.i64 v70
;; @0025                               v73 = iadd v20, v71
;;                                     v180 = iconst.i32 40
;; @0025                               v74 = isub v66, v180  ; v180 = 40
;; @0025                               v75 = uextend.i64 v74
;; @0025                               v76 = isub v73, v75
;; @0025                               store user2 little region0 v3, v76
;; @0025                               v83 = load.i32 user2 readonly region0 v24
;; @0025                               v77 = iconst.i32 2
;;                                     v186 = icmp ugt v83, v77  ; v77 = 2
;; @0025                               trapz v186, user17
;; @0025                               v86 = uextend.i64 v83
;;                                     v188 = ishl v86, v117  ; v117 = 3
;; @0025                               v89 = ushr v188, v11  ; v11 = 32
;; @0025                               trapnz v89, user2
;;                                     v195 = ishl v83, v6  ; v6 = 3
;; @0025                               v92 = uadd_overflow_trap v195, v7, user2  ; v7 = 32
;; @0025                               v96 = uadd_overflow_trap v19, v92, user2
;; @0025                               v97 = uextend.i64 v96
;; @0025                               v99 = iadd v20, v97
;;                                     v213 = iconst.i32 48
;; @0025                               v100 = isub v92, v213  ; v213 = 48
;; @0025                               v101 = uextend.i64 v100
;; @0025                               v102 = isub v99, v101
;; @0025                               store user2 little region0 v4, v102
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v19
;; }
