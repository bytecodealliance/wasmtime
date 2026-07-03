;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
;;! test = "optimize"
(module
  (type $ty (array (mut i8)))

  (func (param (ref $ty) i32) (result i32)
    (array.get_u $ty (local.get 0) (local.get 1))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 67108896 "VMStoreContext+0x20"
;;     region3 = 67108904 "VMStoreContext+0x28"
;;     region4 = 536870912 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0022                               trapz v2, user16
;; @0022                               v5 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0022                               v6 = load.i64 notrap aligned readonly can_move region2 v5+32
;; @0022                               v4 = uextend.i64 v2
;; @0022                               v7 = iadd v6, v4
;; @0022                               v8 = iconst.i64 16
;; @0022                               v9 = iadd v7, v8  ; v8 = 16
;; @0022                               v10 = load.i32 user2 readonly region4 v9
;; @0022                               v11 = icmp ult v3, v10
;; @0022                               trapz v11, user17
;; @0022                               v13 = uextend.i64 v10
;; @0022                               v15 = iconst.i64 32
;; @0022                               v16 = ushr v13, v15  ; v15 = 32
;; @0022                               trapnz v16, user2
;; @0022                               v18 = iconst.i32 20
;; @0022                               v19 = uadd_overflow_trap v10, v18, user2  ; v18 = 20
;; @0022                               v23 = uadd_overflow_trap v2, v19, user2
;; @0022                               v24 = uextend.i64 v23
;; @0022                               v27 = iadd v6, v24
;; @0022                               v22 = iadd v3, v18  ; v18 = 20
;; @0022                               v28 = isub v19, v22
;; @0022                               v29 = uextend.i64 v28
;; @0022                               v30 = isub v27, v29
;; @0022                               v31 = load.i8 user2 little region4 v30
;; @0025                               jump block1
;;
;;                                 block1:
;; @0022                               v32 = uextend.i32 v31
;; @0025                               return v32
;; }
