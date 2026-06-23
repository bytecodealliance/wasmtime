;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $s (struct))
  (func (param anyref) (result (ref $s))
    (ref.cast (ref $s) (local.get 0))
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
;; @001e                               v3 = iconst.i32 0
;; @001e                               v4 = icmp eq v2, v3  ; v3 = 0
;; @001e                               brif v4, block4(v3), block2  ; v3 = 0
;;
;;                                 block2:
;; @001e                               v7 = iconst.i32 1
;; @001e                               v8 = band.i32 v2, v7  ; v7 = 1
;;                                     v22 = iconst.i32 0
;; @001e                               brif v8, block4(v22), block3  ; v22 = 0
;;
;;                                 block3:
;; @001e                               v13 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @001e                               v14 = load.i64 notrap aligned readonly can_move region3 v13+32
;; @001e                               v12 = uextend.i64 v2
;; @001e                               v15 = iadd v14, v12
;; @001e                               v16 = iconst.i64 4
;; @001e                               v17 = iadd v15, v16  ; v16 = 4
;; @001e                               v18 = load.i32 user2 readonly region5 v17
;; @001e                               v10 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @001e                               v11 = load.i32 notrap aligned readonly can_move v10
;; @001e                               v19 = icmp eq v18, v11
;; @001e                               v20 = uextend.i32 v19
;; @001e                               jump block4(v20)
;;
;;                                 block4(v21: i32):
;; @001e                               trapz v21, user19
;; @0021                               jump block1
;;
;;                                 block1:
;; @0021                               return v2
;; }
