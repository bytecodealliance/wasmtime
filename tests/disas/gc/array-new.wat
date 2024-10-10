;;! target = "x86_64"
;;! flags = "-W function-references,gc"
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
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32 uext, i32 uext, i32 uext, i32 uext) -> i32 system_v
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0022                               v6 = uextend.i64 v3
;;                                     v38 = iconst.i64 3
;;                                     v39 = ishl v6, v38  ; v38 = 3
;;                                     v37 = iconst.i64 32
;; @0022                               v8 = ushr v39, v37  ; v37 = 32
;; @0022                               trapnz v8, user18
;; @0022                               v5 = iconst.i32 24
;;                                     v45 = iconst.i32 3
;;                                     v46 = ishl v3, v45  ; v45 = 3
;; @0022                               v10 = uadd_overflow_trap v5, v46, user18  ; v5 = 24
;; @0022                               v12 = iconst.i32 -1476395008
;; @0022                               v13 = iconst.i32 0
;;                                     v43 = iconst.i32 8
;; @0022                               v15 = call fn0(v0, v12, v13, v10, v43)  ; v12 = -1476395008, v13 = 0, v43 = 8
;; @0022                               v19 = uextend.i64 v15
;; @0022                               v20 = iconst.i64 16
;; @0022                               v21 = uadd_overflow_trap v19, v20, user1  ; v20 = 16
;; @0022                               v22 = uextend.i64 v10
;; @0022                               v23 = uadd_overflow_trap v19, v22, user1
;; @0022                               v18 = load.i64 notrap aligned readonly v0+48
;; @0022                               v24 = icmp ule v23, v18
;; @0022                               trapz v24, user1
;; @0022                               v17 = load.i64 notrap aligned readonly v0+40
;; @0022                               v25 = iadd v17, v21
;; @0022                               store notrap aligned v3, v25
;;                                     v36 = iconst.i64 8
;; @0022                               v27 = iadd v25, v36  ; v36 = 8
;;                                     v51 = iconst.i64 -16
;;                                     v63 = iadd v22, v51  ; v51 = -16
;;                                     v65 = iadd v25, v63
;; @0022                               jump block2(v27)
;;
;;                                 block2(v33: i64):
;; @0022                               v34 = icmp eq v33, v65
;; @0022                               brif v34, block4, block3
;;
;;                                 block3:
;; @0022                               store.i64 notrap aligned little v2, v33
;;                                     v66 = iconst.i64 8
;;                                     v67 = iadd.i64 v33, v66  ; v66 = 8
;; @0022                               jump block2(v67)
;;
;;                                 block4:
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v15
;; }
