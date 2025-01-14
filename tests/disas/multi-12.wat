;;! target = "x86_64"

(module
  (func (export "multiLoop") (param i64 i64 i64) (result i64 i64)
    (local.get 2)
    (local.get 1)
    (local.get 0)
    ;; More params than results.
    (loop (param i64 i64 i64) (result i64 i64)
      drop
      return)))

;; function u0:0(i64 vmctx, i64, i64, i64, i64) -> i64, i64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;; @0031                               jump block2
;;
;;                                 block2:
;; @0034                               return v4, v3
;; }
