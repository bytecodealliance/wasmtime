;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut funcref))))

  (func (param funcref) (result (ref $ty))
    (struct.new $ty (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned gv4+32
;;     gv6 = load.i64 notrap aligned readonly can_move gv4+24
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     sig1 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u1:26 sig0
;;     fn1 = colocated u1:29 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;; @0020                               v9 = load.i64 notrap aligned readonly v0+40
;; @0020                               v10 = load.i32 notrap aligned v9
;;                                     v47 = iconst.i32 7
;; @0020                               v13 = uadd_overflow_trap v10, v47, user18  ; v47 = 7
;;                                     v54 = iconst.i32 -8
;; @0020                               v15 = band v13, v54  ; v54 = -8
;; @0020                               v4 = iconst.i32 16
;; @0020                               v16 = uadd_overflow_trap v15, v4, user18  ; v4 = 16
;; @0020                               v38 = load.i64 notrap aligned readonly can_move v0+8
;; @0020                               v18 = load.i64 notrap aligned v38+32
;; @0020                               v17 = uextend.i64 v16
;; @0020                               v19 = icmp ule v17, v18
;; @0020                               brif v19, block2, block3
;;
;;                                 block2:
;;                                     v55 = iconst.i32 -1342177264
;; @0020                               v23 = load.i64 notrap aligned readonly can_move v38+24
;;                                     v62 = band.i32 v13, v54  ; v54 = -8
;;                                     v63 = uextend.i64 v62
;; @0020                               v25 = iadd v23, v63
;; @0020                               store notrap aligned v55, v25  ; v55 = -1342177264
;; @0020                               v29 = load.i64 notrap aligned readonly can_move v0+48
;; @0020                               v30 = load.i32 notrap aligned readonly can_move v29
;; @0020                               store notrap aligned v30, v25+4
;; @0020                               store.i32 notrap aligned v16, v9
;; @0020                               v33 = call fn1(v0, v2)
;; @0020                               v34 = ireduce.i32 v33
;;                                     v35 = iconst.i64 8
;; @0020                               v31 = iadd v25, v35  ; v35 = 8
;; @0020                               store notrap aligned little v34, v31
;; @0023                               jump block1
;;
;;                                 block3 cold:
;; @0020                               v21 = isub.i64 v17, v18
;; @0020                               v22 = call fn0(v0, v21)
;; @0020                               jump block2
;;
;;                                 block1:
;;                                     v64 = band.i32 v13, v54  ; v54 = -8
;; @0023                               return v64
;; }
