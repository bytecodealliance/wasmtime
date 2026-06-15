;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
;;! test = "optimize"
(module
  (type $ty (array (mut i64)))

  (func (param (ref $ty) i32) (result i64)
    (array.get $ty (local.get 0) (local.get 1))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32) -> i64 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 268435488 "VMStoreContext+0x20"
;;     region3 = 268435496 "VMStoreContext+0x28"
;;     region4 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0022                               trapz v2, user16
;; @0022                               v6 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0022                               v7 = load.i64 notrap aligned readonly can_move region2 v6+32
;; @0022                               v5 = uextend.i64 v2
;; @0022                               v8 = iadd v7, v5
;; @0022                               v9 = iconst.i64 16
;; @0022                               v10 = iadd v8, v9  ; v9 = 16
;; @0022                               v11 = load.i32 user2 readonly region4 v10
;; @0022                               v12 = icmp ult v3, v11
;; @0022                               trapz v12, user17
;; @0022                               v14 = uextend.i64 v11
;;                                     v33 = iconst.i64 3
;;                                     v34 = ishl v14, v33  ; v33 = 3
;; @0022                               v16 = iconst.i64 32
;; @0022                               v17 = ushr v34, v16  ; v16 = 32
;; @0022                               trapnz v17, user2
;;                                     v43 = iconst.i32 3
;;                                     v44 = ishl v11, v43  ; v43 = 3
;; @0022                               v19 = iconst.i32 24
;; @0022                               v20 = uadd_overflow_trap v44, v19, user2  ; v19 = 24
;; @0022                               v24 = uadd_overflow_trap v2, v20, user2
;; @0022                               v25 = uextend.i64 v24
;; @0022                               v28 = iadd v7, v25
;;                                     v50 = ishl v3, v43  ; v43 = 3
;; @0022                               v23 = iadd v50, v19  ; v19 = 24
;; @0022                               v29 = isub v20, v23
;; @0022                               v30 = uextend.i64 v29
;; @0022                               v31 = isub v28, v30
;; @0022                               v32 = load.i64 user2 little region4 v31
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v32
;; }
