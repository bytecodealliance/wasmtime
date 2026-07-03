;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
;;! test = "optimize"
(module
  (type $ty (array (mut i64)))

  (func (param (ref $ty) i32 i64)
    (array.set $ty (local.get 0) (local.get 1) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i64) tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i64):
;; @0024                               trapz v2, user16
;; @0024                               v6 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0024                               v7 = load.i64 notrap aligned readonly can_move region2 v6+32
;; @0024                               v5 = uextend.i64 v2
;; @0024                               v8 = iadd v7, v5
;; @0024                               v9 = iconst.i64 16
;; @0024                               v10 = iadd v8, v9  ; v9 = 16
;; @0024                               v11 = load.i32 user2 readonly region4 v10
;; @0024                               v12 = icmp ult v3, v11
;; @0024                               trapz v12, user17
;; @0024                               v14 = uextend.i64 v11
;;                                     v33 = iconst.i64 3
;;                                     v34 = ishl v14, v33  ; v33 = 3
;; @0024                               v16 = iconst.i64 32
;; @0024                               v17 = ushr v34, v16  ; v16 = 32
;; @0024                               trapnz v17, user2
;;                                     v41 = iconst.i32 3
;;                                     v42 = ishl v11, v41  ; v41 = 3
;; @0024                               v19 = iconst.i32 24
;; @0024                               v20 = uadd_overflow_trap v42, v19, user2  ; v19 = 24
;; @0024                               v24 = uadd_overflow_trap v2, v20, user2
;; @0024                               v25 = uextend.i64 v24
;; @0024                               v28 = iadd v7, v25
;;                                     v47 = ishl v3, v41  ; v41 = 3
;; @0024                               v23 = iadd v47, v19  ; v19 = 24
;; @0024                               v29 = isub v20, v23
;; @0024                               v30 = uextend.i64 v29
;; @0024                               v31 = isub v28, v30
;; @0024                               store user2 little region4 v4, v31
;; @0027                               jump block1
;;
;;                                 block1:
;; @0027                               return
;; }
