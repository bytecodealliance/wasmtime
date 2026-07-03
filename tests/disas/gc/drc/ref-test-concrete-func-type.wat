;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $f (func (param i32) (result i32)))
  (func (param funcref) (result i32)
    (ref.test (ref $f) (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 40 "VMContext+0x28"
;;     region3 = 1677721600 "TypeIdsArray+0x0"
;;     region4 = 1610612752 "VMFuncRef+0x10"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;; @0020                               v3 = iconst.i64 0
;; @0020                               v4 = icmp eq v2, v3  ; v3 = 0
;; @0020                               v6 = iconst.i32 0
;; @0020                               brif v4, block4(v6), block2  ; v6 = 0
;;
;;                                 block2:
;; @0020                               jump block3
;;
;;                                 block3:
;; @0020                               v9 = load.i32 notrap aligned readonly region4 v2+16
;; @0020                               v7 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @0020                               v8 = load.i32 notrap aligned readonly can_move region3 v7
;; @0020                               v10 = icmp eq v9, v8
;; @0020                               v11 = uextend.i32 v10
;; @0020                               jump block4(v11)
;;
;;                                 block4(v12: i32):
;; @0023                               jump block1
;;
;;                                 block1:
;; @0023                               return v12
;; }
