;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $ty (array (mut i64)))

  (func (param (ref $ty)) (result i32)
    (array.len (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001f                               trapz v2, user16
;; @001f                               v4 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @001f                               v5 = load.i64 notrap aligned readonly can_move region2 v4+32
;; @001f                               v3 = uextend.i64 v2
;; @001f                               v6 = iadd v5, v3
;; @001f                               v7 = iconst.i64 24
;; @001f                               v8 = iadd v6, v7  ; v7 = 24
;; @001f                               v9 = load.i32 user2 readonly region4 v8
;; @0021                               jump block1
;;
;;                                 block1:
;; @0021                               return v9
;; }
