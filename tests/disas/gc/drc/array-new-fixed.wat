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
;; @0025                               v16 = iconst.i32 -1476395008
;; @0025                               v18 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v19 = load.i32 notrap aligned readonly can_move v18
;;                                     v129 = iconst.i32 56
;; @0025                               v20 = iconst.i32 8
;; @0025                               v21 = call fn0(v0, v16, v19, v129, v20)  ; v16 = -1476395008, v129 = 56, v20 = 8
;; @0025                               v6 = iconst.i32 3
;; @0025                               v117 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v22 = load.i64 notrap aligned readonly can_move v117+32
;; @0025                               v23 = uextend.i64 v21
;; @0025                               v24 = iadd v22, v23
;;                                     v120 = iconst.i64 24
;; @0025                               v26 = iadd v24, v120  ; v120 = 24
;; @0025                               store user2 region0 v6, v26  ; v6 = 3
;; @0025                               trapz v21, user16
;; @0025                               v46 = uadd_overflow_trap v21, v129, user2  ; v129 = 56
;; @0025                               v47 = uextend.i64 v46
;; @0025                               v49 = iadd v22, v47
;; @0025                               v52 = isub v49, v120  ; v120 = 24
;; @0025                               store user2 little region0 v2, v52
;; @0025                               v59 = load.i32 user2 readonly region0 v26
;; @0025                               v53 = iconst.i32 1
;;                                     v160 = icmp ugt v59, v53  ; v53 = 1
;; @0025                               trapz v160, user17
;; @0025                               v62 = uextend.i64 v59
;;                                     v119 = iconst.i64 3
;;                                     v162 = ishl v62, v119  ; v119 = 3
;; @0025                               v11 = iconst.i64 32
;; @0025                               v65 = ushr v162, v11  ; v11 = 32
;; @0025                               trapnz v65, user2
;;                                     v169 = ishl v59, v6  ; v6 = 3
;; @0025                               v7 = iconst.i32 32
;; @0025                               v68 = uadd_overflow_trap v169, v7, user2  ; v7 = 32
;; @0025                               v72 = uadd_overflow_trap v21, v68, user2
;; @0025                               v73 = uextend.i64 v72
;; @0025                               v75 = iadd v22, v73
;;                                     v182 = iconst.i32 40
;; @0025                               v76 = isub v68, v182  ; v182 = 40
;; @0025                               v77 = uextend.i64 v76
;; @0025                               v78 = isub v75, v77
;; @0025                               store user2 little region0 v3, v78
;; @0025                               v85 = load.i32 user2 readonly region0 v26
;; @0025                               v79 = iconst.i32 2
;;                                     v188 = icmp ugt v85, v79  ; v79 = 2
;; @0025                               trapz v188, user17
;; @0025                               v88 = uextend.i64 v85
;;                                     v190 = ishl v88, v119  ; v119 = 3
;; @0025                               v91 = ushr v190, v11  ; v11 = 32
;; @0025                               trapnz v91, user2
;;                                     v197 = ishl v85, v6  ; v6 = 3
;; @0025                               v94 = uadd_overflow_trap v197, v7, user2  ; v7 = 32
;; @0025                               v98 = uadd_overflow_trap v21, v94, user2
;; @0025                               v99 = uextend.i64 v98
;; @0025                               v101 = iadd v22, v99
;;                                     v215 = iconst.i32 48
;; @0025                               v102 = isub v94, v215  ; v215 = 48
;; @0025                               v103 = uextend.i64 v102
;; @0025                               v104 = isub v101, v103
;; @0025                               store user2 little region0 v4, v104
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v21
;; }
