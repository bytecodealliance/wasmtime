;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
;;! test = "optimize"
(module
  (type $ty (array (mut i64)))

  (func (param (ref $ty) i32 i64 i32)
    (array.fill $ty (local.get 0) (local.get 1) (local.get 2) (local.get 3))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i64, i32) tail {
;;     region0 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i64, v5: i32):
;; @0027                               trapz v2, user16
;; @0027                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @0027                               v7 = load.i64 notrap aligned readonly can_move v49+32
;; @0027                               v6 = uextend.i64 v2
;; @0027                               v8 = iadd v7, v6
;; @0027                               v9 = iconst.i64 16
;; @0027                               v10 = iadd v8, v9  ; v9 = 16
;; @0027                               v11 = load.i32 user2 readonly region0 v10
;; @0027                               v13 = uextend.i64 v3
;; @0027                               v14 = uextend.i64 v5
;; @0027                               v17 = iadd v13, v14
;; @0027                               v12 = uextend.i64 v11
;; @0027                               v18 = icmp ugt v17, v12
;; @0027                               trapnz v18, user17
;; @0027                               v32 = load.i64 notrap aligned v49+40
;; @0027                               v22 = iconst.i64 24
;; @0027                               v23 = iadd v8, v22  ; v22 = 24
;;                                     v53 = iconst.i64 3
;;                                     v54 = ishl v13, v53  ; v53 = 3
;; @0027                               v27 = iadd v23, v54
;;                                     v56 = ishl v14, v53  ; v53 = 3
;; @0027                               v34 = uadd_overflow_trap v27, v56, user2
;; @0027                               v33 = iadd v7, v32
;; @0027                               v35 = icmp ugt v34, v33
;; @0027                               trapnz v35, user2
;;                                     v51 = iconst.i64 0
;; @0027                               v38 = icmp eq v14, v51  ; v51 = 0
;; @0027                               v25 = iconst.i64 8
;; @0027                               v36 = iadd v27, v56
;; @0027                               brif v38, block3, block2(v27)
;;
;;                                 block2(v39: i64):
;; @0027                               store.i64 user2 little region0 v4, v39
;;                                     v58 = iconst.i64 8
;;                                     v59 = iadd v39, v58  ; v58 = 8
;; @0027                               v42 = icmp eq v59, v36
;; @0027                               brif v42, block3, block2(v59)
;;
;;                                 block3:
;; @002a                               jump block1
;;
;;                                 block1:
;; @002a                               return
;; }
