;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $s (struct))
  (func (param anyref) (result i32)
    (ref.test (ref $s) (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 40 "VMContext+0x28"
;;     region3 = 268435488 "VMStoreContext+0x20"
;;     region4 = 268435496 "VMStoreContext+0x28"
;;     region5 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001d                               v4 = iconst.i32 0
;; @001d                               v5 = icmp eq v2, v4  ; v4 = 0
;; @001d                               brif v5, block4(v4), block2  ; v4 = 0
;;
;;                                 block2:
;; @001d                               v8 = iconst.i32 1
;; @001d                               v9 = band.i32 v2, v8  ; v8 = 1
;;                                     v23 = iconst.i32 0
;; @001d                               brif v9, block4(v23), block3  ; v23 = 0
;;
;;                                 block3:
;; @001d                               v14 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @001d                               v15 = load.i64 notrap aligned readonly can_move region3 v14+32
;; @001d                               v13 = uextend.i64 v2
;; @001d                               v16 = iadd v15, v13
;; @001d                               v17 = iconst.i64 4
;; @001d                               v18 = iadd v16, v17  ; v17 = 4
;; @001d                               v19 = load.i32 user2 readonly region5 v18
;; @001d                               v11 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @001d                               v12 = load.i32 notrap aligned readonly can_move v11
;; @001d                               v20 = icmp eq v19, v12
;; @001d                               v21 = uextend.i32 v20
;; @001d                               jump block4(v21)
;;
;;                                 block4(v22: i32):
;; @0020                               jump block1(v22)
;;
;;                                 block1(v3: i32):
;; @0020                               return v3
;; }
