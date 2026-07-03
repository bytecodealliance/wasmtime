;;! target = "x86_64"
;;! test = "optimize"

(module
  (func (;0;) (param i64) (result i64)
    i64.const 32
    i64.const -19
    i64.shr_u
    ;; call 0
  )
)
;; function u0:0(i64 vmctx, i64, i64) -> i64 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;; @001e                               jump block1
;;
;;                                 block1:
;;                                     v6 = iconst.i64 0
;; @001e                               return v6  ; v6 = 0
;; }
