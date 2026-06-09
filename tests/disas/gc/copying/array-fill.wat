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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i64, v5: i32):
;; @0027                               trapz v2, user16
;; @0027                               v46 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0027                               v7 = load.i64 notrap aligned readonly can_move v46+32
;; @0027                               v6 = uextend.i64 v2
;; @0027                               v8 = iadd v7, v6
;; @0027                               v9 = iconst.i64 16
;; @0027                               v10 = iadd v8, v9  ; v9 = 16
;; @0027                               v11 = load.i32 user2 readonly region1 v10
;; @0027                               v13 = uextend.i64 v3
;; @0027                               v14 = uextend.i64 v5
;; @0027                               v17 = iadd v13, v14
;; @0027                               v12 = uextend.i64 v11
;; @0027                               v18 = icmp ugt v17, v12
;; @0027                               trapnz v18, user17
;; @0027                               v35 = load.i64 notrap aligned v46+40
;; @0027                               v23 = iconst.i64 24
;; @0027                               v24 = iadd v8, v23  ; v23 = 24
;;                                     v50 = iconst.i64 3
;;                                     v51 = ishl v13, v50  ; v50 = 3
;; @0027                               v28 = iadd v24, v51
;;                                     v53 = ishl v14, v50  ; v50 = 3
;; @0027                               v37 = uadd_overflow_trap v28, v53, user2
;; @0027                               v36 = iadd v7, v35
;; @0027                               v38 = icmp ugt v37, v36
;; @0027                               trapnz v38, user2
;;                                     v48 = iconst.i64 0
;; @0027                               v41 = icmp eq v14, v48  ; v48 = 0
;; @0027                               v26 = iconst.i64 8
;; @0027                               v39 = iadd v28, v53
;; @0027                               brif v41, block3, block2(v28)
;;
;;                                 block2(v42: i64):
;; @0027                               store.i64 user2 little region1 v4, v42
;;                                     v55 = iconst.i64 8
;;                                     v56 = iadd v42, v55  ; v55 = 8
;; @0027                               v45 = icmp eq v56, v39
;; @0027                               brif v45, block3, block2(v56)
;;
;;                                 block3:
;; @002a                               jump block1
;;
;;                                 block1:
;; @002a                               return
;; }
