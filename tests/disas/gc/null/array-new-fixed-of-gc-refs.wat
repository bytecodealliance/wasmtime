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
;;                                     v127 = stack_addr.i64 ss2
;;                                     store notrap v2, v127
;;                                     v128 = stack_addr.i64 ss1
;;                                     store notrap v3, v128
;;                                     v129 = stack_addr.i64 ss0
;;                                     store notrap v4, v129
;; @0025                               v19 = load.i64 notrap aligned readonly region0 v0+32
;; @0025                               v20 = load.i32 user2 region1 v19
;;                                     v163 = iconst.i32 7
;; @0025                               v23 = uadd_overflow_trap v20, v163, user18  ; v163 = 7
;;                                     v169 = iconst.i32 -8
;; @0025                               v25 = band v23, v169  ; v169 = -8
;;                                     v156 = iconst.i32 24
;; @0025                               v26 = uadd_overflow_trap v25, v156, user18  ; v156 = 24
;; @0025                               v144 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v28 = load.i64 notrap aligned v144+40
;; @0025                               v27 = uextend.i64 v26
;; @0025                               v29 = icmp ule v27, v28
;; @0025                               brif v29, block2, block3
;;
;;                                 block2:
;;                                     v170 = iconst.i32 -1476394984
;; @0025                               v33 = load.i64 notrap aligned readonly can_move v144+32
;;                                     v268 = band.i32 v23, v169  ; v169 = -8
;;                                     v269 = uextend.i64 v268
;; @0025                               v35 = iadd v33, v269
;; @0025                               store user2 region1 v170, v35  ; v170 = -1476394984
;; @0025                               v39 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v40 = load.i32 notrap aligned readonly can_move v39
;; @0025                               store user2 region1 v40, v35+4
;; @0025                               store.i32 user2 region1 v26, v19
;; @0025                               v6 = iconst.i32 3
;; @0025                               v41 = iconst.i64 8
;; @0025                               v42 = iadd v35, v41  ; v41 = 8
;; @0025                               store user2 region1 v6, v42  ; v6 = 3
;; @0025                               trapz v268, user16
;;                                     v270 = iconst.i32 24
;; @0025                               v62 = uadd_overflow_trap v268, v270, user2  ; v270 = 24
;;                                     v126 = load.i32 notrap v127
;; @0025                               v63 = uextend.i64 v62
;; @0025                               v65 = iadd v33, v63
;;                                     v147 = iconst.i64 12
;; @0025                               v68 = isub v65, v147  ; v147 = 12
;; @0025                               store user2 little region1 v126, v68
;; @0025                               v75 = load.i32 user2 readonly region1 v42
;; @0025                               v69 = iconst.i32 1
;;                                     v208 = icmp ugt v75, v69  ; v69 = 1
;; @0025                               trapz v208, user17
;; @0025                               v78 = uextend.i64 v75
;;                                     v148 = iconst.i64 2
;;                                     v210 = ishl v78, v148  ; v148 = 2
;; @0025                               v11 = iconst.i64 32
;; @0025                               v81 = ushr v210, v11  ; v11 = 32
;; @0025                               trapnz v81, user2
;;                                     v187 = iconst.i32 2
;;                                     v217 = ishl v75, v187  ; v187 = 2
;; @0025                               v7 = iconst.i32 12
;; @0025                               v84 = uadd_overflow_trap v217, v7, user2  ; v7 = 12
;; @0025                               v88 = uadd_overflow_trap v268, v84, user2
;;                                     v124 = load.i32 notrap v128
;; @0025                               v89 = uextend.i64 v88
;; @0025                               v91 = iadd v33, v89
;;                                     v230 = iconst.i32 16
;; @0025                               v92 = isub v84, v230  ; v230 = 16
;; @0025                               v93 = uextend.i64 v92
;; @0025                               v94 = isub v91, v93
;; @0025                               store user2 little region1 v124, v94
;; @0025                               v101 = load.i32 user2 readonly region1 v42
;;                                     v236 = icmp ugt v101, v187  ; v187 = 2
;; @0025                               trapz v236, user17
;; @0025                               v104 = uextend.i64 v101
;;                                     v238 = ishl v104, v148  ; v148 = 2
;; @0025                               v107 = ushr v238, v11  ; v11 = 32
;; @0025                               trapnz v107, user2
;;                                     v245 = ishl v101, v187  ; v187 = 2
;; @0025                               v110 = uadd_overflow_trap v245, v7, user2  ; v7 = 12
;; @0025                               v114 = uadd_overflow_trap v268, v110, user2
;;                                     v122 = load.i32 notrap v129
;; @0025                               v115 = uextend.i64 v114
;; @0025                               v117 = iadd v33, v115
;;                                     v262 = iconst.i32 20
;; @0025                               v118 = isub v110, v262  ; v262 = 20
;; @0025                               v119 = uextend.i64 v118
;; @0025                               v120 = isub v117, v119
;; @0025                               store user2 little region1 v122, v120
;; @0029                               jump block1
;;
;;                                 block3 cold:
;; @0025                               v31 = isub.i64 v27, v28
;; @0025                               v32 = call fn0(v0, v31), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]
;; @0025                               jump block2
;;
;;                                 block1:
;;                                     v271 = band.i32 v23, v169  ; v169 = -8
;; @0029                               return v271
;; }
