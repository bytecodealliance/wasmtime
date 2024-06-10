;;! target = "x86_64"

(module
  (func (export "foo")
    i32.const 1
    ;; Fewer params than results.
    (block (param i32) (result i32 i64)
      i64.const 2
    )
    drop
    drop
  )
)

;; function u0:0(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0026                               v2 = iconst.i32 1
;; @002a                               v5 = iconst.i64 2
;; @002c                               jump block2(v2, v5)  ; v2 = 1, v5 = 2
;;
;;                                 block2(v3: i32, v4: i64):
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               return
;; }
