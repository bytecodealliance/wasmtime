;;! target = "x86_64"

(module
  (func (export "multiLoop") (param i64 i64) (result i64 i64)
    (local.get 1)
    (local.get 0)
    (loop (param i64 i64) (result i64 i64)
       return)))

;; function u0:0(i64 vmctx, i64, i64, i64) -> i64, i64 tail {
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64):
;; @002e                               jump block2(v3, v2)
;;
;;                                 block2(v6: i64, v7: i64):
;; @0030                               return v6, v7
;; }
