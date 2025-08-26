;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $ty (array (mut i64)))

  (func (param i64 i32) (result (ref $ty))
    (array.new $ty (local.get 0) (local.get 1))
  )
)
;; function u0:0(i64 vmctx, i64, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u1610612736:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0022                               v6 = uextend.i64 v3
;;                                     v35 = iconst.i64 3
;;                                     v36 = ishl v6, v35  ; v35 = 3
;;                                     v33 = iconst.i64 32
;; @0022                               v8 = ushr v36, v33  ; v33 = 32
;; @0022                               trapnz v8, user18
;; @0022                               v5 = iconst.i32 32
;;                                     v42 = iconst.i32 3
;;                                     v43 = ishl v3, v42  ; v42 = 3
;; @0022                               v10 = uadd_overflow_trap v5, v43, user18  ; v5 = 32
;; @0022                               v12 = iconst.i32 -1476395008
;; @0022                               v13 = iconst.i32 0
;;                                     v40 = iconst.i32 8
;; @0022                               v15 = call fn0(v0, v12, v13, v10, v40)  ; v12 = -1476395008, v13 = 0, v40 = 8
;; @0022                               v31 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v16 = load.i64 notrap aligned readonly can_move v31+24
;; @0022                               v17 = uextend.i64 v15
;; @0022                               v18 = iadd v16, v17
;;                                     v30 = iconst.i64 24
;; @0022                               v19 = iadd v18, v30  ; v30 = 24
;; @0022                               store notrap aligned v3, v19
;;                                     v52 = iadd v18, v33  ; v33 = 32
;; @0022                               v25 = uextend.i64 v10
;; @0022                               v26 = iadd v18, v25
;;                                     v34 = iconst.i64 8
;; @0022                               jump block2(v52)
;;
;;                                 block2(v27: i64):
;; @0022                               v28 = icmp eq v27, v26
;; @0022                               brif v28, block4, block3
;;
;;                                 block3:
;; @0022                               store.i64 notrap aligned little v2, v27
;;                                     v64 = iconst.i64 8
;;                                     v65 = iadd.i64 v27, v64  ; v64 = 8
;; @0022                               jump block2(v65)
;;
;;                                 block4:
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v15
;; }
