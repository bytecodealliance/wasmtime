;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
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
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v130 = stack_addr.i64 ss2
;;                                     store notrap v2, v130
;;                                     v131 = stack_addr.i64 ss1
;;                                     store notrap v3, v131
;;                                     v132 = stack_addr.i64 ss0
;;                                     store notrap v4, v132
;; @0025                               v15 = load.i64 notrap aligned readonly can_move v0+32
;; @0025                               v16 = load.i32 notrap aligned v15
;; @0025                               v17 = load.i32 notrap aligned v15+4
;; @0025                               v23 = uextend.i64 v16
;; @0025                               v11 = iconst.i64 32
;; @0025                               v24 = iadd v23, v11  ; v11 = 32
;; @0025                               v25 = uextend.i64 v17
;; @0025                               v26 = icmp ule v24, v25
;; @0025                               brif v26, block2, block3
;;
;;                                 block2:
;;                                     v268 = iconst.i32 32
;;                                     v174 = iadd.i32 v16, v268  ; v268 = 32
;; @0025                               store notrap aligned region0 v174, v15
;;                                     v269 = iconst.i32 -1476394994
;;                                     v270 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v271 = load.i64 notrap aligned readonly can_move v270+32
;; @0025                               v38 = iadd v271, v23
;; @0025                               store notrap aligned v269, v38  ; v269 = -1476394994
;;                                     v272 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v273 = load.i32 notrap aligned readonly can_move v272
;; @0025                               store notrap aligned v273, v38+4
;;                                     v274 = iconst.i64 32
;; @0025                               istore32 notrap aligned v274, v38+8  ; v274 = 32
;; @0025                               jump block4(v16, v38)
;;
;;                                 block3 cold:
;; @0025                               v27 = iconst.i32 -1476394994
;; @0025                               v28 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v29 = load.i32 notrap aligned readonly can_move v28
;;                                     v160 = iconst.i32 32
;; @0025                               v30 = iconst.i32 16
;; @0025                               v31 = call fn0(v0, v27, v29, v160, v30), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v27 = -1476394994, v160 = 32, v30 = 16
;; @0025                               v145 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v32 = load.i64 notrap aligned readonly can_move v145+32
;; @0025                               v33 = uextend.i64 v31
;; @0025                               v34 = iadd v32, v33
;; @0025                               jump block4(v31, v34)
;;
;;                                 block4(v42: i32, v43: i64):
;; @0025                               v6 = iconst.i32 3
;; @0025                               v44 = iconst.i64 16
;; @0025                               v45 = iadd v43, v44  ; v44 = 16
;; @0025                               store user2 region1 v6, v45  ; v6 = 3
;; @0025                               trapz v42, user16
;;                                     v275 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v276 = load.i64 notrap aligned readonly can_move v275+32
;; @0025                               v47 = uextend.i64 v42
;; @0025                               v49 = iadd v276, v47
;; @0025                               v51 = iadd v49, v44  ; v44 = 16
;; @0025                               v52 = load.i32 user2 readonly region1 v51
;; @0025                               trapz v52, user17
;; @0025                               v55 = uextend.i64 v52
;;                                     v151 = iconst.i64 2
;;                                     v180 = ishl v55, v151  ; v151 = 2
;;                                     v277 = iconst.i64 32
;;                                     v278 = ushr v180, v277  ; v277 = 32
;; @0025                               trapnz v278, user2
;;                                     v189 = iconst.i32 2
;;                                     v190 = ishl v52, v189  ; v189 = 2
;; @0025                               v7 = iconst.i32 20
;; @0025                               v61 = uadd_overflow_trap v190, v7, user2  ; v7 = 20
;; @0025                               v65 = uadd_overflow_trap v42, v61, user2
;;                                     v129 = load.i32 notrap v130
;; @0025                               v66 = uextend.i64 v65
;; @0025                               v68 = iadd v276, v66
;; @0025                               v69 = isub v61, v7  ; v7 = 20
;; @0025                               v70 = uextend.i64 v69
;; @0025                               v71 = isub v68, v70
;; @0025                               store user2 little region1 v129, v71
;; @0025                               v78 = load.i32 user2 readonly region1 v51
;; @0025                               v72 = iconst.i32 1
;;                                     v207 = icmp ugt v78, v72  ; v72 = 1
;; @0025                               trapz v207, user17
;; @0025                               v81 = uextend.i64 v78
;;                                     v209 = ishl v81, v151  ; v151 = 2
;;                                     v279 = ushr v209, v277  ; v277 = 32
;; @0025                               trapnz v279, user2
;;                                     v216 = ishl v78, v189  ; v189 = 2
;; @0025                               v87 = uadd_overflow_trap v216, v7, user2  ; v7 = 20
;; @0025                               v91 = uadd_overflow_trap v42, v87, user2
;;                                     v127 = load.i32 notrap v131
;; @0025                               v92 = uextend.i64 v91
;; @0025                               v94 = iadd v276, v92
;;                                     v229 = iconst.i32 24
;; @0025                               v95 = isub v87, v229  ; v229 = 24
;; @0025                               v96 = uextend.i64 v95
;; @0025                               v97 = isub v94, v96
;; @0025                               store user2 little region1 v127, v97
;; @0025                               v104 = load.i32 user2 readonly region1 v51
;;                                     v235 = icmp ugt v104, v189  ; v189 = 2
;; @0025                               trapz v235, user17
;; @0025                               v107 = uextend.i64 v104
;;                                     v237 = ishl v107, v151  ; v151 = 2
;;                                     v280 = ushr v237, v277  ; v277 = 32
;; @0025                               trapnz v280, user2
;;                                     v244 = ishl v104, v189  ; v189 = 2
;; @0025                               v113 = uadd_overflow_trap v244, v7, user2  ; v7 = 20
;; @0025                               v117 = uadd_overflow_trap v42, v113, user2
;;                                     v125 = load.i32 notrap v132
;; @0025                               v118 = uextend.i64 v117
;; @0025                               v120 = iadd v276, v118
;;                                     v262 = iconst.i32 28
;; @0025                               v121 = isub v113, v262  ; v262 = 28
;; @0025                               v122 = uextend.i64 v121
;; @0025                               v123 = isub v120, v122
;; @0025                               store user2 little region1 v125, v123
;; @0029                               jump block1(v42)
;;
;;                                 block1(v5: i32):
;; @0029                               return v5
;; }
