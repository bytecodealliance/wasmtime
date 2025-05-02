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
;;                                 block0(v0: i64, v1: i64):
;; @0026                               v2 = iconst.i32 1
;; @0028                               v3 = iconst.i64 2
;; @002d                               jump block2
;;
;;                                 block2:
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               return
;; }
