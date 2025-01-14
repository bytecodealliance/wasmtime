;;! target = "x86_64"

(module
  (func (export "f") (param i64 i32) (result i64)
    (local.get 0)
    (local.get 1)
    (local.get 1)
    ;; If with else. More params than results.
    (if (param i64 i32) (result i64)
      (then
        (drop)
        (drop)
        (i64.const -1))
      (else
        (drop)
        (drop)
        (i64.const -2)))))

;; function u0:0(i64 vmctx, i64, i64, i32) -> i64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0027                               brif v3, block2, block4
;;
;;                                 block2:
;; @002b                               v8 = iconst.i64 -1
;; @002d                               jump block3(v8)  ; v8 = -1
;;
;;                                 block4:
;; @0030                               v9 = iconst.i64 -2
;; @0032                               jump block3(v9)  ; v9 = -2
;;
;;                                 block3(v5: i64):
;; @0033                               jump block1(v5)
;;
;;                                 block1(v4: i64):
;; @0033                               return v4
;; }
