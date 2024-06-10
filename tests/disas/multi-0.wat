;;! target = "x86_64"

(module
  (func (export "i64.dup") (param i64) (result i64 i64)
    (local.get 0) (local.get 0)))

;; function u0:0(i64 vmctx, i64, i64) -> i64, i64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;; @002b                               jump block1(v2, v2)
;;
;;                                 block1(v3: i64, v4: i64):
;; @002b                               return v3, v4
;; }
