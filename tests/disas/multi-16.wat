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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0024                               v4 = iconst.i32 1
;; @0028                               brif v2, block2, block4(v4)  ; v4 = 1
;;
;;                                 block2:
;; @002a                               v6 = iconst.i32 2
;; @002c                               v7 = iadd.i32 v4, v6  ; v4 = 1, v6 = 2
;; @002d                               jump block3(v7)
;;
;;                                 block4(v8: i32):
;; @002e                               v9 = iconst.i32 -2
;; @0030                               v10 = iadd.i32 v4, v9  ; v4 = 1, v9 = -2
;; @0031                               jump block3(v10)
;;
;;                                 block3(v5: i32):
;; @0032                               jump block1(v5)
;;
;;                                 block1(v3: i32):
;; @0032                               return v3
;; }
