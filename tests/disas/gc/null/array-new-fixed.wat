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
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;; @0025                               v17 = load.i64 notrap aligned readonly v0+40
;; @0025                               v18 = load.i32 notrap aligned v17
;;                                     v73 = iconst.i32 7
;; @0025                               v21 = uadd_overflow_trap v18, v73, user18  ; v73 = 7
;;                                     v80 = iconst.i32 -8
;; @0025                               v23 = band v21, v80  ; v80 = -8
;;                                     v65 = iconst.i32 40
;; @0025                               v24 = uadd_overflow_trap v23, v65, user18  ; v65 = 40
;; @0025                               v50 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v26 = load.i64 notrap aligned v50+32
;; @0025                               v25 = uextend.i64 v24
;; @0025                               v27 = icmp ule v25, v26
;; @0025                               brif v27, block2, block3
;;
;;                                 block2:
;;                                     v81 = iconst.i32 -1476394968
;; @0025                               v31 = load.i64 notrap aligned readonly can_move v50+24
;;                                     v118 = band.i32 v21, v80  ; v80 = -8
;;                                     v119 = uextend.i64 v118
;; @0025                               v33 = iadd v31, v119
;; @0025                               store notrap aligned v81, v33  ; v81 = -1476394968
;; @0025                               v37 = load.i64 notrap aligned readonly can_move v0+48
;; @0025                               v38 = load.i32 notrap aligned readonly can_move v37
;; @0025                               store notrap aligned v38, v33+4
;; @0025                               store.i32 notrap aligned v24, v17
;; @0025                               v6 = iconst.i32 3
;;                                     v53 = iconst.i64 8
;; @0025                               v39 = iadd v33, v53  ; v53 = 8
;; @0025                               store notrap aligned v6, v39  ; v6 = 3
;;                                     v89 = iconst.i64 16
;;                                     v95 = iadd v33, v89  ; v89 = 16
;; @0025                               store.i64 notrap aligned little v2, v95
;;                                     v55 = iconst.i64 24
;;                                     v102 = iadd v33, v55  ; v55 = 24
;; @0025                               store.i64 notrap aligned little v3, v102
;;                                     v52 = iconst.i64 32
;;                                     v109 = iadd v33, v52  ; v52 = 32
;; @0025                               store.i64 notrap aligned little v4, v109
;; @0029                               jump block1
;;
;;                                 block3 cold:
;; @0025                               v29 = isub.i64 v25, v26
;; @0025                               v30 = call fn0(v0, v29)
;; @0025                               jump block2
;;
;;                                 block1:
;;                                     v120 = band.i32 v21, v80  ; v80 = -8
;; @0029                               return v120
;; }
