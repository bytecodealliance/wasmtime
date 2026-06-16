;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (array (mut i64)))

  (func (param (ref $ty) i32 i32) (result i64 i64)
    (array.get $ty (local.get 0) (local.get 1))
    (array.get $ty (local.get 0) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32) -> i64, i64 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0024                               trapz v2, user16
;; @0024                               v8 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0024                               v9 = load.i64 notrap aligned readonly can_move v8+32
;; @0024                               v7 = uextend.i64 v2
;; @0024                               v10 = iadd v9, v7
;; @0024                               v11 = iconst.i64 8
;; @0024                               v12 = iadd v10, v11  ; v11 = 8
;; @0024                               v13 = load.i32 user2 readonly region1 v12
;; @0024                               v14 = icmp ult v3, v13
;; @0024                               trapz v14, user17
;; @0024                               v16 = uextend.i64 v13
;;                                     v63 = iconst.i64 3
;;                                     v64 = ishl v16, v63  ; v63 = 3
;; @0024                               v18 = iconst.i64 32
;; @0024                               v19 = ushr v64, v18  ; v18 = 32
;; @0024                               trapnz v19, user2
;;                                     v73 = iconst.i32 3
;;                                     v74 = ishl v13, v73  ; v73 = 3
;; @0024                               v21 = iconst.i32 16
;; @0024                               v22 = uadd_overflow_trap v74, v21, user2  ; v21 = 16
;; @0024                               v26 = uadd_overflow_trap v2, v22, user2
;; @0024                               v27 = uextend.i64 v26
;; @0024                               v30 = iadd v9, v27
;;                                     v80 = ishl v3, v73  ; v73 = 3
;; @0024                               v25 = iadd v80, v21  ; v21 = 16
;; @0024                               v31 = isub v22, v25
;; @0024                               v32 = uextend.i64 v31
;; @0024                               v33 = isub v30, v32
;; @0024                               v34 = load.i64 user2 little region1 v33
;; @002b                               v42 = icmp ult v4, v13
;; @002b                               trapz v42, user17
;;                                     v82 = ishl v4, v73  ; v73 = 3
;; @002b                               v53 = iadd v82, v21  ; v21 = 16
;; @002b                               v59 = isub v22, v53
;; @002b                               v60 = uextend.i64 v59
;; @002b                               v61 = isub v30, v60
;; @002b                               v62 = load.i64 user2 little region1 v61
;; @002e                               jump block1
;;
;;                                 block1:
;; @002e                               return v34, v62
;; }
