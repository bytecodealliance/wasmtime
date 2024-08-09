;;! target = "x86_64"

;; Reachable `if` head and unreachable consequent and alternative means that the
;; following block is unreachable.

(module
  (func (param i32) (result i32)
    local.get 0
    if
      unreachable
    else
      unreachable
    end
    i32.const 0))

;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001b                               brif v2, block2, block4
;;
;;                                 block2:
;; @001d                               trap unreachable
;;
;;                                 block4:
;; @001f                               trap unreachable
;; }
