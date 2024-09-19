;;! target = "x86_64"
;;! flags = "-W function-references,gc"
;;! test = "optimize"

(module
  (type $ty (array (mut i64)))

  (func (param (ref $ty) i32 i64 i32)
    (array.fill $ty (local.get 0) (local.get 1) (local.get 2) (local.get 3))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i64, v5: i32):
;; @0027                               trapz v2, null_reference
;; @0027                               v9 = uextend.i64 v2
;; @0027                               v10 = iconst.i64 16
;; @0027                               v11 = uadd_overflow_trap v9, v10, user65535  ; v10 = 16
;; @0027                               v12 = iconst.i64 4
;; @0027                               v13 = uadd_overflow_trap v11, v12, user65535  ; v12 = 4
;; @0027                               v8 = load.i64 notrap aligned readonly v0+48
;; @0027                               v14 = icmp ule v13, v8
;; @0027                               trapz v14, user65535
;; @0027                               v7 = load.i64 notrap aligned readonly v0+40
;; @0027                               v15 = iadd v7, v11
;; @0027                               v16 = load.i32 notrap aligned v15
;; @0027                               v17 = uadd_overflow_trap v3, v5, array_oob
;; @0027                               v18 = icmp ugt v17, v16
;; @0027                               trapnz v18, array_oob
;; @0027                               v20 = uextend.i64 v16
;;                                     v47 = iconst.i64 3
;;                                     v48 = ishl v20, v47  ; v47 = 3
;;                                     v46 = iconst.i64 32
;; @0027                               v22 = ushr v48, v46  ; v46 = 32
;; @0027                               trapnz v22, user65535
;;                                     v57 = iconst.i32 3
;;                                     v58 = ishl v16, v57  ; v57 = 3
;; @0027                               v24 = iconst.i32 24
;; @0027                               v25 = uadd_overflow_trap v58, v24, user65535  ; v24 = 24
;;                                     v65 = ishl v3, v57  ; v57 = 3
;;                                     v67 = iadd v65, v24  ; v24 = 24
;; @0027                               v33 = uextend.i64 v67
;; @0027                               v34 = uadd_overflow_trap v9, v33, user65535
;; @0027                               v35 = uextend.i64 v25
;; @0027                               v36 = uadd_overflow_trap v9, v35, user65535
;; @0027                               v37 = icmp ule v36, v8
;; @0027                               trapz v37, user65535
;; @0027                               v38 = iadd v7, v34
;; @0027                               v40 = uextend.i64 v65
;; @0027                               v41 = iadd v38, v40
;; @0027                               v19 = iconst.i64 8
;; @0027                               jump block2(v38)
;;
;;                                 block2(v42: i64):
;; @0027                               v43 = icmp eq v42, v41
;; @0027                               brif v43, block4, block3
;;
;;                                 block3:
;; @0027                               store.i64 notrap aligned little v4, v42
;;                                     v69 = iconst.i64 8
;;                                     v70 = iadd.i64 v42, v69  ; v69 = 8
;; @0027                               jump block2(v70)
;;
;;                                 block4:
;; @002a                               jump block1
;;
;;                                 block1:
;; @002a                               return
;; }
