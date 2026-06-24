;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (func (param anyref) (result i32)
    (ref.test (ref i31) (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001e                               jump block1
;;
;;                                 block1:
;; @001b                               v3 = iconst.i32 1
;; @001b                               v4 = band.i32 v2, v3  ; v3 = 1
;; @001e                               return v4
;; }
