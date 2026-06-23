;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (func (param anyref) (result i32)
    (ref.test (ref eq) (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001b                               v3 = iconst.i32 0
;; @001b                               v4 = icmp eq v2, v3  ; v3 = 0
;; @001b                               brif v4, block4(v3), block2  ; v3 = 0
;;
;;                                 block2:
;; @001b                               v7 = iconst.i32 1
;; @001b                               v8 = band.i32 v2, v7  ; v7 = 1
;; @001b                               brif v8, block4(v7), block3  ; v7 = 1
;;
;;                                 block3:
;; @001b                               v11 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @001b                               v12 = load.i64 notrap aligned readonly can_move region2 v11+32
;; @001b                               v10 = uextend.i64 v2
;; @001b                               v13 = iadd v12, v10
;; @001b                               v16 = load.i32 user2 readonly region4 v13
;; @001b                               v17 = iconst.i32 -1610612736
;; @001b                               v18 = band v16, v17  ; v17 = -1610612736
;; @001b                               v19 = icmp eq v18, v17  ; v17 = -1610612736
;; @001b                               v20 = uextend.i32 v19
;; @001b                               jump block4(v20)
;;
;;                                 block4(v21: i32):
;; @001e                               jump block1
;;
;;                                 block1:
;; @001e                               return v21
;; }
