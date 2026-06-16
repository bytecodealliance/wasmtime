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
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001f                               trapz v2, user16
;; @001f                               v5 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @001f                               v6 = load.i64 notrap aligned readonly can_move v5+32
;; @001f                               v4 = uextend.i64 v2
;; @001f                               v7 = iadd v6, v4
;; @001f                               v8 = iconst.i64 24
;; @001f                               v9 = iadd v7, v8  ; v8 = 24
;; @001f                               v10 = load.i32 user2 readonly region1 v9
;; @0021                               jump block1
;;
;;                                 block1:
;; @0021                               return v10
;; }
