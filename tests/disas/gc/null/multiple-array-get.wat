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
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 67108896 "VMStoreContext+0x20"
;;     region3 = 67108904 "VMStoreContext+0x28"
;;     region4 = 536870912 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0024                               trapz v2, user16
;; @0024                               v6 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0024                               v7 = load.i64 notrap aligned readonly can_move region2 v6+32
;; @0024                               v5 = uextend.i64 v2
;; @0024                               v8 = iadd v7, v5
;; @0024                               v9 = iconst.i64 8
;; @0024                               v10 = iadd v8, v9  ; v9 = 8
;; @0024                               v11 = load.i32 user2 readonly region4 v10
;; @0024                               v12 = icmp ult v3, v11
;; @0024                               trapz v12, user17
;; @0024                               v14 = uextend.i64 v11
;;                                     v62 = iconst.i64 3
;;                                     v63 = ishl v14, v62  ; v62 = 3
;; @0024                               v16 = iconst.i64 32
;; @0024                               v17 = ushr v63, v16  ; v16 = 32
;; @0024                               trapnz v17, user2
;;                                     v70 = iconst.i32 3
;;                                     v71 = ishl v11, v70  ; v70 = 3
;; @0024                               v19 = iconst.i32 16
;; @0024                               v20 = uadd_overflow_trap v71, v19, user2  ; v19 = 16
;; @0024                               v24 = uadd_overflow_trap v2, v20, user2
;; @0024                               v25 = uextend.i64 v24
;; @0024                               v28 = iadd v7, v25
;;                                     v76 = ishl v3, v70  ; v70 = 3
;; @0024                               v23 = iadd v76, v19  ; v19 = 16
;; @0024                               v29 = isub v20, v23
;; @0024                               v30 = uextend.i64 v29
;; @0024                               v31 = isub v28, v30
;; @0024                               v32 = load.i64 user2 little region4 v31
;; @002b                               v40 = icmp ult v4, v11
;; @002b                               trapz v40, user17
;;                                     v78 = ishl v4, v70  ; v70 = 3
;; @002b                               v51 = iadd v78, v19  ; v19 = 16
;; @002b                               v57 = isub v20, v51
;; @002b                               v58 = uextend.i64 v57
;; @002b                               v59 = isub v28, v58
;; @002b                               v60 = load.i64 user2 little region4 v59
;; @002e                               jump block1
;;
;;                                 block1:
;; @002e                               return v32, v60
;; }
