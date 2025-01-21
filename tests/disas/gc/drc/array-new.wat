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
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i64 tail
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0022                               v6 = uextend.i64 v3
;;                                     v35 = iconst.i64 3
;;                                     v36 = ishl v6, v35  ; v35 = 3
;;                                     v33 = iconst.i64 32
;; @0022                               v8 = ushr v36, v33  ; v33 = 32
;; @0022                               trapnz v8, user18
;; @0022                               v5 = iconst.i32 24
;;                                     v42 = iconst.i32 3
;;                                     v43 = ishl v3, v42  ; v42 = 3
;; @0022                               v10 = uadd_overflow_trap v5, v43, user18  ; v5 = 24
;; @0022                               v12 = iconst.i32 -1476395008
;; @0022                               v13 = iconst.i32 0
;;                                     v40 = iconst.i32 8
;; @0022                               v15 = call fn0(v0, v12, v13, v10, v40)  ; v12 = -1476395008, v13 = 0, v40 = 8
;; @0022                               v18 = load.i64 notrap aligned readonly v0+40
;; @0022                               v16 = ireduce.i32 v15
;; @0022                               v19 = uextend.i64 v16
;; @0022                               v20 = iadd v18, v19
;;                                     v34 = iconst.i64 16
;; @0022                               v21 = iadd v20, v34  ; v34 = 16
;; @0022                               store notrap aligned v3, v21
;;                                     v47 = iconst.i64 24
;;                                     v53 = iadd v20, v47  ; v47 = 24
;; @0022                               v27 = uextend.i64 v10
;; @0022                               v28 = iadd v20, v27
;;                                     v32 = iconst.i64 8
;; @0022                               jump block2(v53)
;;
;;                                 block2(v29: i64):
;; @0022                               v30 = icmp eq v29, v28
;; @0022                               brif v30, block4, block3
;;
;;                                 block3:
;; @0022                               store.i64 notrap aligned little v2, v29
;;                                     v65 = iconst.i64 8
;;                                     v66 = iadd.i64 v29, v65  ; v65 = 8
;; @0022                               jump block2(v66)
;;
;;                                 block4:
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v16
;; }
