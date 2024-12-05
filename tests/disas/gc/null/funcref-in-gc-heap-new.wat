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
;;     sig0 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u1:28 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;;                                     v35 = iconst.i32 0
;; @0020                               trapnz v35, user18  ; v35 = 0
;; @0020                               v9 = load.i64 notrap aligned readonly v0+56
;; @0020                               v10 = load.i32 notrap aligned v9
;;                                     v42 = iconst.i32 7
;; @0020                               v13 = uadd_overflow_trap v10, v42, user18  ; v42 = 7
;;                                     v49 = iconst.i32 -8
;; @0020                               v15 = band v13, v49  ; v49 = -8
;; @0020                               v4 = iconst.i32 16
;; @0020                               v16 = uadd_overflow_trap v15, v4, user18  ; v4 = 16
;; @0020                               v17 = uextend.i64 v16
;; @0020                               v21 = load.i64 notrap aligned readonly v0+48
;; @0020                               v22 = icmp ule v17, v21
;; @0020                               trapz v22, user18
;;                                     v50 = iconst.i32 -1342177264
;; @0020                               v19 = load.i64 notrap aligned readonly v0+40
;; @0020                               v23 = uextend.i64 v15
;; @0020                               v24 = iadd v19, v23
;; @0020                               store notrap aligned v50, v24  ; v50 = -1342177264
;; @0020                               v28 = load.i64 notrap aligned readonly v0+80
;; @0020                               v29 = load.i32 notrap aligned readonly v28
;; @0020                               store notrap aligned v29, v24+4
;; @0020                               store notrap aligned v16, v9
;; @0020                               v32 = call fn0(v0, v2)
;; @0020                               v33 = ireduce.i32 v32
;;                                     v34 = iconst.i64 8
;; @0020                               v30 = iadd v24, v34  ; v34 = 8
;; @0020                               store notrap aligned little v33, v30
;; @0023                               jump block1
;;
;;                                 block1:
;;                                     v57 = band.i32 v13, v49  ; v49 = -8
;; @0023                               return v57
;; }
