;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
;;! test = "optimize"
(module
  (func (param anyref) (result i32)
    (ref.test (ref struct) (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001b                               v4 = iconst.i32 0
;; @001b                               v5 = icmp eq v2, v4  ; v4 = 0
;; @001b                               brif v5, block4(v4), block2  ; v4 = 0
;;
;;                                 block2:
;; @001b                               v8 = iconst.i32 1
;; @001b                               v9 = band.i32 v2, v8  ; v8 = 1
;;                                     v23 = iconst.i32 0
;; @001b                               brif v9, block4(v23), block3  ; v23 = 0
;;
;;                                 block3:
;; @001b                               v12 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @001b                               v13 = load.i64 notrap aligned readonly can_move v12+32
;; @001b                               v11 = uextend.i64 v2
;; @001b                               v14 = iadd v13, v11
;; @001b                               v17 = load.i32 user2 readonly region1 v14
;; @001b                               v18 = iconst.i32 -1342177280
;; @001b                               v19 = band v17, v18  ; v18 = -1342177280
;; @001b                               v20 = icmp eq v19, v18  ; v18 = -1342177280
;; @001b                               v21 = uextend.i32 v20
;; @001b                               jump block4(v21)
;;
;;                                 block4(v22: i32):
;; @001e                               jump block1(v22)
;;
;;                                 block1(v3: i32):
;; @001e                               return v3
;; }
