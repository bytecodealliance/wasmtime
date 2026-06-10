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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 40 "VMContext+0x28"
;;     region2 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;; @0025                               v15 = iconst.i32 -1476395008
;; @0025                               v16 = load.i64 notrap aligned readonly can_move region1 v0+40
;; @0025                               v17 = load.i32 notrap aligned readonly can_move v16
;;                                     v126 = iconst.i32 56
;; @0025                               v18 = iconst.i32 8
;; @0025                               v19 = call fn0(v0, v15, v17, v126, v18)  ; v15 = -1476395008, v126 = 56, v18 = 8
;; @0025                               v6 = iconst.i32 3
;; @0025                               v20 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0025                               v21 = load.i64 notrap aligned readonly can_move v20+32
;; @0025                               v22 = uextend.i64 v19
;; @0025                               v23 = iadd v21, v22
;;                                     v117 = iconst.i64 24
;; @0025                               v25 = iadd v23, v117  ; v117 = 24
;; @0025                               store user2 region2 v6, v25  ; v6 = 3
;; @0025                               trapz v19, user16
;; @0025                               v45 = uadd_overflow_trap v19, v126, user2  ; v126 = 56
;; @0025                               v46 = uextend.i64 v45
;; @0025                               v48 = iadd v21, v46
;; @0025                               v51 = isub v48, v117  ; v117 = 24
;; @0025                               store user2 little region2 v2, v51
;; @0025                               v58 = load.i32 user2 readonly region2 v25
;; @0025                               v52 = iconst.i32 1
;;                                     v157 = icmp ugt v58, v52  ; v52 = 1
;; @0025                               trapz v157, user17
;; @0025                               v61 = uextend.i64 v58
;;                                     v116 = iconst.i64 3
;;                                     v159 = ishl v61, v116  ; v116 = 3
;; @0025                               v11 = iconst.i64 32
;; @0025                               v64 = ushr v159, v11  ; v11 = 32
;; @0025                               trapnz v64, user2
;;                                     v166 = ishl v58, v6  ; v6 = 3
;; @0025                               v7 = iconst.i32 32
;; @0025                               v67 = uadd_overflow_trap v166, v7, user2  ; v7 = 32
;; @0025                               v71 = uadd_overflow_trap v19, v67, user2
;; @0025                               v72 = uextend.i64 v71
;; @0025                               v74 = iadd v21, v72
;;                                     v179 = iconst.i32 40
;; @0025                               v75 = isub v67, v179  ; v179 = 40
;; @0025                               v76 = uextend.i64 v75
;; @0025                               v77 = isub v74, v76
;; @0025                               store user2 little region2 v3, v77
;; @0025                               v84 = load.i32 user2 readonly region2 v25
;; @0025                               v78 = iconst.i32 2
;;                                     v185 = icmp ugt v84, v78  ; v78 = 2
;; @0025                               trapz v185, user17
;; @0025                               v87 = uextend.i64 v84
;;                                     v187 = ishl v87, v116  ; v116 = 3
;; @0025                               v90 = ushr v187, v11  ; v11 = 32
;; @0025                               trapnz v90, user2
;;                                     v194 = ishl v84, v6  ; v6 = 3
;; @0025                               v93 = uadd_overflow_trap v194, v7, user2  ; v7 = 32
;; @0025                               v97 = uadd_overflow_trap v19, v93, user2
;; @0025                               v98 = uextend.i64 v97
;; @0025                               v100 = iadd v21, v98
;;                                     v212 = iconst.i32 48
;; @0025                               v101 = isub v93, v212  ; v212 = 48
;; @0025                               v102 = uextend.i64 v101
;; @0025                               v103 = isub v100, v102
;; @0025                               store user2 little region2 v4, v103
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v19
;; }
