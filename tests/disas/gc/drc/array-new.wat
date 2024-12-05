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
;;     sig0 = (i64 vmctx, i32 uext, i32 uext, i32 uext, i32 uext) -> i32 tail
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0022                               v6 = uextend.i64 v3
;;                                     v34 = iconst.i64 3
;;                                     v35 = ishl v6, v34  ; v34 = 3
;;                                     v32 = iconst.i64 32
;; @0022                               v8 = ushr v35, v32  ; v32 = 32
;; @0022                               trapnz v8, user18
;; @0022                               v5 = iconst.i32 24
;;                                     v41 = iconst.i32 3
;;                                     v42 = ishl v3, v41  ; v41 = 3
;; @0022                               v10 = uadd_overflow_trap v5, v42, user18  ; v5 = 24
;; @0022                               v12 = iconst.i32 -1476395008
;; @0022                               v13 = iconst.i32 0
;;                                     v39 = iconst.i32 8
;; @0022                               v15 = call fn0(v0, v12, v13, v10, v39)  ; v12 = -1476395008, v13 = 0, v39 = 8
;; @0022                               v17 = load.i64 notrap aligned readonly v0+40
;; @0022                               v18 = uextend.i64 v15
;; @0022                               v19 = iadd v17, v18
;;                                     v33 = iconst.i64 16
;; @0022                               v20 = iadd v19, v33  ; v33 = 16
;; @0022                               store notrap aligned v3, v20
;;                                     v46 = iconst.i64 24
;;                                     v52 = iadd v19, v46  ; v46 = 24
;; @0022                               v26 = uextend.i64 v10
;; @0022                               v27 = iadd v19, v26
;;                                     v31 = iconst.i64 8
;; @0022                               jump block2(v52)
;;
;;                                 block2(v28: i64):
;; @0022                               v29 = icmp eq v28, v27
;; @0022                               brif v29, block4, block3
;;
;;                                 block3:
;; @0022                               store.i64 notrap aligned little v2, v28
;;                                     v64 = iconst.i64 8
;;                                     v65 = iadd.i64 v28, v64  ; v64 = 8
;; @0022                               jump block2(v65)
;;
;;                                 block4:
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v15
;; }
