;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $s (struct))
  (func (param anyref) (result i32)
    (ref.test (ref $s) (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 40 "VMContext+0x28"
;;     region3 = 1677721600 "TypeIdsArray+0x0"
;;     region4 = 67108896 "VMStoreContext+0x20"
;;     region5 = 67108904 "VMStoreContext+0x28"
;;     region6 = 536870912 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001d                               v3 = iconst.i32 0
;; @001d                               v4 = icmp eq v2, v3  ; v3 = 0
;; @001d                               brif v4, block4(v3), block2  ; v3 = 0
;;
;;                                 block2:
;; @001d                               v7 = iconst.i32 1
;; @001d                               v8 = band.i32 v2, v7  ; v7 = 1
;;                                     v22 = iconst.i32 0
;; @001d                               brif v8, block4(v22), block3  ; v22 = 0
;;
;;                                 block3:
;; @001d                               v13 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @001d                               v14 = load.i64 notrap aligned readonly can_move region4 v13+32
;; @001d                               v12 = uextend.i64 v2
;; @001d                               v15 = iadd v14, v12
;; @001d                               v16 = iconst.i64 4
;; @001d                               v17 = iadd v15, v16  ; v16 = 4
;; @001d                               v18 = load.i32 user2 readonly region6 v17
;; @001d                               v10 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @001d                               v11 = load.i32 notrap aligned readonly can_move region3 v10
;; @001d                               v19 = icmp eq v18, v11
;; @001d                               v20 = uextend.i32 v19
;; @001d                               jump block4(v20)
;;
;;                                 block4(v21: i32):
;; @0020                               jump block1
;;
;;                                 block1:
;; @0020                               return v21
;; }
