;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
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
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i64, v5: i32):
;; @0027                               trapz v2, user16
;; @0027                               v10 = uextend.i64 v2
;; @0027                               v11 = iconst.i64 16
;; @0027                               v12 = uadd_overflow_trap v10, v11, user1  ; v11 = 16
;; @0027                               v13 = iconst.i64 4
;; @0027                               v14 = uadd_overflow_trap v12, v13, user1  ; v13 = 4
;; @0027                               v9 = load.i64 notrap aligned readonly v0+48
;; @0027                               v15 = icmp ule v14, v9
;; @0027                               trapz v15, user1
;; @0027                               v7 = load.i64 notrap aligned readonly v0+40
;; @0027                               v16 = iadd v7, v12
;; @0027                               v17 = load.i32 notrap aligned v16
;; @0027                               v18 = uadd_overflow_trap v3, v5, user17
;; @0027                               v19 = icmp ugt v18, v17
;; @0027                               trapnz v19, user17
;; @0027                               v21 = uextend.i64 v17
;;                                     v48 = iconst.i64 3
;;                                     v49 = ishl v21, v48  ; v48 = 3
;;                                     v47 = iconst.i64 32
;; @0027                               v23 = ushr v49, v47  ; v47 = 32
;; @0027                               trapnz v23, user1
;;                                     v58 = iconst.i32 3
;;                                     v59 = ishl v17, v58  ; v58 = 3
;; @0027                               v25 = iconst.i32 24
;; @0027                               v26 = uadd_overflow_trap v59, v25, user1  ; v25 = 24
;;                                     v66 = ishl v3, v58  ; v58 = 3
;;                                     v68 = iadd v66, v25  ; v25 = 24
;; @0027                               v35 = uextend.i64 v68
;; @0027                               v36 = uadd_overflow_trap v10, v35, user1
;; @0027                               v37 = uextend.i64 v26
;; @0027                               v38 = uadd_overflow_trap v10, v37, user1
;; @0027                               v39 = icmp ule v38, v9
;; @0027                               trapz v39, user1
;; @0027                               v40 = iadd v7, v36
;; @0027                               v41 = uextend.i64 v66
;; @0027                               v42 = iadd v40, v41
;; @0027                               v20 = iconst.i64 8
;; @0027                               jump block2(v40)
;;
;;                                 block2(v44: i64):
;; @0027                               v45 = icmp eq v44, v42
;; @0027                               brif v45, block4, block3
;;
;;                                 block3:
;; @0027                               store.i64 notrap aligned little v4, v44
;;                                     v70 = iconst.i64 8
;;                                     v71 = iadd.i64 v44, v70  ; v70 = 8
;; @0027                               jump block2(v71)
;;
;;                                 block4:
;; @002a                               jump block1
;;
;;                                 block1:
;; @002a                               return
;; }
