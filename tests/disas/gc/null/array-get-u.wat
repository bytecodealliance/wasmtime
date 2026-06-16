;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (array (mut i8)))

  (func (param (ref $ty) i32) (result i32)
    (array.get_u $ty (local.get 0) (local.get 1))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0022                               trapz v2, user16
;; @0022                               v6 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0022                               v7 = load.i64 notrap aligned readonly can_move v6+32
;; @0022                               v5 = uextend.i64 v2
;; @0022                               v8 = iadd v7, v5
;; @0022                               v9 = iconst.i64 8
;; @0022                               v10 = iadd v8, v9  ; v9 = 8
;; @0022                               v11 = load.i32 user2 readonly region1 v10
;; @0022                               v12 = icmp ult v3, v11
;; @0022                               trapz v12, user17
;; @0022                               v14 = uextend.i64 v11
;; @0022                               v16 = iconst.i64 32
;; @0022                               v17 = ushr v14, v16  ; v16 = 32
;; @0022                               trapnz v17, user2
;; @0022                               v19 = iconst.i32 12
;; @0022                               v20 = uadd_overflow_trap v11, v19, user2  ; v19 = 12
;; @0022                               v24 = uadd_overflow_trap v2, v20, user2
;; @0022                               v25 = uextend.i64 v24
;; @0022                               v28 = iadd v7, v25
;; @0022                               v23 = iadd v3, v19  ; v19 = 12
;; @0022                               v29 = isub v20, v23
;; @0022                               v30 = uextend.i64 v29
;; @0022                               v31 = isub v28, v30
;; @0022                               v32 = load.i8 user2 little region1 v31
;; @0025                               jump block1
;;
;;                                 block1:
;; @0022                               v33 = uextend.i32 v32
;; @0025                               return v33
;; }
