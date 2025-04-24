;;! target = "x86_64"

(module
  (func (export "multiLoop") (param i64) (result i64 i64)
    (local.get 0)
    ;; Fewer params than results.
    (loop (param i64) (result i64 i64)
      i64.const 42
      return)))

;; function u0:0(i64 vmctx, i64, i64) -> i64, i64 tail {
;;                                 block0(v0: i64, v1: i64, v2: i64):
;; @002b                               jump block2
;;
;;                                 block2:
;; @002d                               v8 = iconst.i64 42
;; @002f                               return v2, v8  ; v8 = 42
;; }
