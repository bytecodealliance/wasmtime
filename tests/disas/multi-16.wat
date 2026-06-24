;;! target = "x86_64"

(module
  (func (export "param") (param i32) (result i32)
    (i32.const 1)
    (if (param i32) (result i32) (local.get 0)
      (then (i32.const 2) (i32.add))
      (else (i32.const -2) (i32.add))
    )
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
;; @0024                               v3 = iconst.i32 1
;; @0028                               brif v2, block2, block4
;;
;;                                 block2:
;; @002a                               v4 = iconst.i32 2
;; @002c                               v5 = iadd.i32 v3, v4  ; v3 = 1, v4 = 2
;; @002d                               jump block3(v5)
;;
;;                                 block4:
;; @002e                               v6 = iconst.i32 -2
;; @0030                               v7 = iadd.i32 v3, v6  ; v3 = 1, v6 = -2
;; @0031                               jump block3(v7)
;;
;;                                 block3(v8: i32):
;; @0032                               jump block1
;;
;;                                 block1:
;; @0032                               return v8
;; }
