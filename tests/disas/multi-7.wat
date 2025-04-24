;;! target = "x86_64"

(module
  (func (export "f") (param i64 i32) (result i64)
    (local.get 0)
    (local.get 1)
    ;; If with no else. Same number of params and results.
    (if (param i64) (result i64)
      (then
        (drop)
        (i64.const -1)))))

;; function u0:0(i64 vmctx, i64, i64, i32) -> i64 tail {
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @002a                               brif v3, block2, block3(v2)
;;
;;                                 block2:
;; @002d                               v6 = iconst.i64 -1
;; @002f                               jump block3(v6)  ; v6 = -1
;;
;;                                 block3(v5: i64):
;; @0030                               jump block1(v5)
;;
;;                                 block1(v4: i64):
;; @0030                               return v4
;; }
