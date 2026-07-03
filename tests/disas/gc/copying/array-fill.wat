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
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 67108896 "VMStoreContext+0x20"
;;     region3 = 67108904 "VMStoreContext+0x28"
;;     region4 = 536870912 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i64, v5: i32):
;; @0027                               trapz v2, user16
;; @0027                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0027                               v8 = load.i64 notrap aligned readonly can_move region2 v7+32
;; @0027                               v6 = uextend.i64 v2
;; @0027                               v9 = iadd v8, v6
;; @0027                               v10 = iconst.i64 16
;; @0027                               v11 = iadd v9, v10  ; v10 = 16
;; @0027                               v12 = load.i32 user2 readonly region4 v11
;; @0027                               v14 = uextend.i64 v3
;; @0027                               v15 = uextend.i64 v5
;; @0027                               v18 = iadd v14, v15
;; @0027                               v13 = uextend.i64 v12
;; @0027                               v19 = icmp ugt v18, v13
;; @0027                               trapnz v19, user17
;; @0027                               v36 = load.i64 notrap aligned region3 v7+40
;; @0027                               v24 = iconst.i64 24
;; @0027                               v25 = iadd v9, v24  ; v24 = 24
;;                                     v49 = iconst.i64 3
;;                                     v50 = ishl v14, v49  ; v49 = 3
;; @0027                               v29 = iadd v25, v50
;;                                     v52 = ishl v15, v49  ; v49 = 3
;; @0027                               v38 = uadd_overflow_trap v29, v52, user2
;; @0027                               v37 = iadd v8, v36
;; @0027                               v39 = icmp ugt v38, v37
;; @0027                               trapnz v39, user2
;;                                     v47 = iconst.i64 0
;; @0027                               v42 = icmp eq v15, v47  ; v47 = 0
;; @0027                               v27 = iconst.i64 8
;; @0027                               v40 = iadd v29, v52
;; @0027                               brif v42, block3, block2(v29)
;;
;;                                 block2(v43: i64):
;; @0027                               store.i64 user2 little region4 v4, v43
;;                                     v54 = iconst.i64 8
;;                                     v55 = iadd v43, v54  ; v54 = 8
;; @0027                               v46 = icmp eq v55, v40
;; @0027                               brif v46, block3, block2(v55)
;;
;;                                 block3:
;; @002a                               jump block1
;;
;;                                 block1:
;; @002a                               return
;; }
