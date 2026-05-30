;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $ty (array (mut i64)))

  (func (param (ref $ty) i32) (result i64)
    (array.get $ty (local.get 0) (local.get 1))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32) -> i64 tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0022                               trapz v2, user16
;; @0022                               v33 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v6 = load.i64 notrap aligned readonly can_move v33+32
;; @0022                               v5 = uextend.i64 v2
;; @0022                               v7 = iadd v6, v5
;; @0022                               v8 = iconst.i64 24
;; @0022                               v9 = iadd v7, v8  ; v8 = 24
;; @0022                               v10 = load.i32 user2 readonly region0 v9
;; @0022                               v11 = icmp ult v3, v10
;; @0022                               trapz v11, user17
;; @0022                               v13 = uextend.i64 v10
;;                                     v35 = iconst.i64 3
;;                                     v36 = ishl v13, v35  ; v35 = 3
;; @0022                               v15 = iconst.i64 32
;; @0022                               v16 = ushr v36, v15  ; v15 = 32
;; @0022                               trapnz v16, user2
;;                                     v45 = iconst.i32 3
;;                                     v46 = ishl v10, v45  ; v45 = 3
;; @0022                               v18 = iconst.i32 32
;; @0022                               v19 = uadd_overflow_trap v46, v18, user2  ; v18 = 32
;; @0022                               v23 = uadd_overflow_trap v2, v19, user2
;; @0022                               v24 = uextend.i64 v23
;; @0022                               v26 = iadd v6, v24
;;                                     v52 = ishl v3, v45  ; v45 = 3
;; @0022                               v22 = iadd v52, v18  ; v18 = 32
;; @0022                               v27 = isub v19, v22
;; @0022                               v28 = uextend.i64 v27
;; @0022                               v29 = isub v26, v28
;; @0022                               v30 = load.i64 user2 little region0 v29
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v30
;; }
