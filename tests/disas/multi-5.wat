;;! target = "x86_64"

(module
  (func (export "foo")
    i32.const 1
    i64.const 2
    ;; More params than results.
    (block (param i32 i64) (result i32)
      drop
    )
    drop
  )
)

;; function u0:0(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0026                               v2 = iconst.i32 1
;; @0028                               v3 = iconst.i64 2
;; @002d                               jump block2(v2)  ; v2 = 1
;;
;;                                 block2(v4: i32):
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               return
;; }
