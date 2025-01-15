;;! target = "x86_64"

(module
  (func (export "as-if-else") (param i32 i32) (result i32)
    (block (result i32)
      (if (result i32) (local.get 0)
        (then (local.get 1))
        (else (br 1 (i32.const 4)))
      )
    )
  )
)

;; function u0:0(i64 vmctx, i64, i32, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @002e                               brif v2, block3, block5
;;
;;                                 block3:
;; @0032                               jump block4
;;
;;                                 block5:
;; @0033                               v7 = iconst.i32 4
;; @0035                               jump block2(v7)  ; v7 = 4
;;
;;                                 block4:
;; @0038                               jump block2(v3)
;;
;;                                 block2(v5: i32):
;; @0039                               jump block1(v5)
;;
;;                                 block1(v4: i32):
;; @0039                               return v4
;; }
