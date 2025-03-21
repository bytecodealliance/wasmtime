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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v59 = iconst.i64 0
;; @0025                               trapnz v59, user18  ; v59 = 0
;; @0025                               v7 = iconst.i32 12
;; @0025                               v12 = uadd_overflow_trap v7, v7, user18  ; v7 = 12, v7 = 12
;; @0025                               v14 = iconst.i32 -134217728
;; @0025                               v15 = band v12, v14  ; v14 = -134217728
;; @0025                               trapnz v15, user18
;; @0025                               v17 = load.i64 notrap aligned readonly v0+56
;; @0025                               v18 = load.i32 notrap aligned v17
;;                                     v60 = iconst.i32 7
;; @0025                               v21 = uadd_overflow_trap v18, v60, user18  ; v60 = 7
;;                                     v67 = iconst.i32 -8
;; @0025                               v23 = band v21, v67  ; v67 = -8
;; @0025                               v24 = uadd_overflow_trap v23, v12, user18
;; @0025                               v25 = uextend.i64 v24
;; @0025                               v29 = load.i64 notrap aligned readonly can_move v0+48
;; @0025                               v30 = icmp ule v25, v29
;; @0025                               trapz v30, user18
;; @0025                               v33 = iconst.i32 -1476395008
;;                                     v68 = bor v12, v33  ; v33 = -1476395008
;; @0025                               v27 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v31 = uextend.i64 v23
;; @0025                               v32 = iadd v27, v31
;; @0025                               store notrap aligned v68, v32
;; @0025                               v36 = load.i64 notrap aligned readonly can_move v0+64
;; @0025                               v37 = load.i32 notrap aligned readonly can_move v36
;; @0025                               store notrap aligned v37, v32+4
;; @0025                               store notrap aligned v24, v17
;; @0025                               v6 = iconst.i32 3
;;                                     v48 = iconst.i64 8
;; @0025                               v38 = iadd v32, v48  ; v48 = 8
;; @0025                               store notrap aligned v6, v38  ; v6 = 3
;;                                     v50 = iconst.i64 12
;;                                     v76 = iadd v32, v50  ; v50 = 12
;; @0025                               store notrap aligned little v2, v76
;;                                     v78 = iconst.i64 16
;;                                     v84 = iadd v32, v78  ; v78 = 16
;; @0025                               store notrap aligned little v3, v84
;;                                     v86 = iconst.i64 20
;;                                     v92 = iadd v32, v86  ; v86 = 20
;; @0025                               store notrap aligned little v4, v92
;; @0029                               jump block1
;;
;;                                 block1:
;;                                     v101 = band.i32 v21, v67  ; v67 = -8
;; @0029                               return v101
;; }
