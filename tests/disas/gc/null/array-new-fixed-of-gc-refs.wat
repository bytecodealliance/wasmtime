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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned gv4+32
;;     gv6 = load.i64 notrap aligned readonly can_move gv4+24
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v62 = stack_addr.i64 ss2
;;                                     store notrap v2, v62
;;                                     v61 = stack_addr.i64 ss1
;;                                     store notrap v3, v61
;;                                     v60 = stack_addr.i64 ss0
;;                                     store notrap v4, v60
;; @0025                               v17 = load.i64 notrap aligned readonly v0+32
;; @0025                               v18 = load.i32 notrap aligned v17
;;                                     v82 = iconst.i32 7
;; @0025                               v21 = uadd_overflow_trap v18, v82, user18  ; v82 = 7
;;                                     v89 = iconst.i32 -8
;; @0025                               v23 = band v21, v89  ; v89 = -8
;;                                     v74 = iconst.i32 24
;; @0025                               v24 = uadd_overflow_trap v23, v74, user18  ; v74 = 24
;; @0025                               v56 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v26 = load.i64 notrap aligned v56+32
;; @0025                               v25 = uextend.i64 v24
;; @0025                               v27 = icmp ule v25, v26
;; @0025                               brif v27, block2, block3
;;
;;                                 block2:
;;                                     v90 = iconst.i32 -1476394984
;; @0025                               v31 = load.i64 notrap aligned readonly can_move v56+24
;;                                     v128 = band.i32 v21, v89  ; v89 = -8
;;                                     v129 = uextend.i64 v128
;; @0025                               v33 = iadd v31, v129
;; @0025                               store notrap aligned v90, v33  ; v90 = -1476394984
;; @0025                               v37 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v38 = load.i32 notrap aligned readonly can_move v37
;; @0025                               store notrap aligned v38, v33+4
;; @0025                               store.i32 notrap aligned v24, v17
;; @0025                               v6 = iconst.i32 3
;;                                     v53 = iconst.i64 8
;; @0025                               v39 = iadd v33, v53  ; v53 = 8
;; @0025                               store notrap aligned v6, v39  ; v6 = 3
;;                                     v49 = load.i32 notrap v62
;;                                     v64 = iconst.i64 12
;;                                     v103 = iadd v33, v64  ; v64 = 12
;; @0025                               store notrap aligned little v49, v103
;;                                     v48 = load.i32 notrap v61
;;                                     v105 = iconst.i64 16
;;                                     v111 = iadd v33, v105  ; v105 = 16
;; @0025                               store notrap aligned little v48, v111
;;                                     v47 = load.i32 notrap v60
;;                                     v113 = iconst.i64 20
;;                                     v119 = iadd v33, v113  ; v113 = 20
;; @0025                               store notrap aligned little v47, v119
;; @0029                               jump block1
;;
;;                                 block3 cold:
;; @0025                               v29 = isub.i64 v25, v26
;; @0025                               v30 = call fn0(v0, v29), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]
;; @0025                               jump block2
;;
;;                                 block1:
;;                                     v130 = band.i32 v21, v89  ; v89 = -8
;; @0029                               return v130
;; }
