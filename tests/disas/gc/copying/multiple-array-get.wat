;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
;;! test = "optimize"
(module
  (type $ty (array (mut i64)))

  (func (param (ref $ty) i32 i32) (result i64 i64)
    (array.get $ty (local.get 0) (local.get 1))
    (array.get $ty (local.get 0) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32) -> i64, i64 tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0024                               trapz v2, user16
;; @0024                               v65 = load.i64 notrap aligned readonly can_move v0+8
;; @0024                               v8 = load.i64 notrap aligned readonly can_move v65+32
;; @0024                               v7 = uextend.i64 v2
;; @0024                               v9 = iadd v8, v7
;; @0024                               v10 = iconst.i64 16
;; @0024                               v11 = iadd v9, v10  ; v10 = 16
;; @0024                               v12 = load.i32 user2 readonly region0 v11
;; @0024                               v13 = icmp ult v3, v12
;; @0024                               trapz v13, user17
;; @0024                               v15 = uextend.i64 v12
;;                                     v67 = iconst.i64 3
;;                                     v68 = ishl v15, v67  ; v67 = 3
;; @0024                               v17 = iconst.i64 32
;; @0024                               v18 = ushr v68, v17  ; v17 = 32
;; @0024                               trapnz v18, user2
;;                                     v77 = iconst.i32 3
;;                                     v78 = ishl v12, v77  ; v77 = 3
;; @0024                               v20 = iconst.i32 24
;; @0024                               v21 = uadd_overflow_trap v78, v20, user2  ; v20 = 24
;; @0024                               v25 = uadd_overflow_trap v2, v21, user2
;; @0024                               v26 = uextend.i64 v25
;; @0024                               v28 = iadd v8, v26
;;                                     v84 = ishl v3, v77  ; v77 = 3
;; @0024                               v24 = iadd v84, v20  ; v20 = 24
;; @0024                               v29 = isub v21, v24
;; @0024                               v30 = uextend.i64 v29
;; @0024                               v31 = isub v28, v30
;; @0024                               v32 = load.i64 user2 little region0 v31
;; @002b                               v39 = icmp ult v4, v12
;; @002b                               trapz v39, user17
;;                                     v86 = ishl v4, v77  ; v77 = 3
;; @002b                               v50 = iadd v86, v20  ; v20 = 24
;; @002b                               v55 = isub v21, v50
;; @002b                               v56 = uextend.i64 v55
;; @002b                               v57 = isub v28, v56
;; @002b                               v58 = load.i64 user2 little region0 v57
;; @002e                               jump block1
;;
;;                                 block1:
;; @002e                               return v32, v58
;; }
