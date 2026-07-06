;;! target = "x86_64"

;; A `loop` that never branches back to the loop header only ever runs a single
;; iteration, so its header block has just one predecessor and should have no
;; block parameters.

(module
  (func (param i32) (result i32)
    local.get 0
    (loop (param i32) (result i32)
      i32.const 1
      i32.add
    )
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001b                               jump block2
;;
;;                                 block2:
;; @001d                               v4 = iconst.i32 1
;; @001f                               v5 = iadd.i32 v2, v4  ; v4 = 1
;; @0020                               jump block3
;;
;;                                 block3:
;; @0021                               jump block1
;;
;;                                 block1:
;; @0021                               return v5
;; }
