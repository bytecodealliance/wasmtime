;;! target = "x86_64"
;;! test = "optimize"

(module
  ;; This function body should ideally get compiled down into a single `trapz`
  ;; CLIF instruction.
  (func (export "trapnz") (param i32)
    local.get 0
    if
      unreachable
    end
  )

  ;; And this one into a single `trapnz` instruction.
  (func (export "trapz") (param i32)
    local.get 0
    i32.eqz
    if
      unreachable
    end
  )
)

;; function u0:0(i64 vmctx, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @002f                               brif v2, block2, block3
;;
;;                                 block2:
;; @0031                               trap user11
;;
;;                                 block3:
;; @0033                               jump block1
;;
;;                                 block1:
;; @0033                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v5 = iconst.i32 0
;; @0038                               v3 = icmp eq v2, v5  ; v5 = 0
;; @0038                               v4 = uextend.i32 v3
;; @0039                               brif v4, block2, block3
;;
;;                                 block2:
;; @003b                               trap user11
;;
;;                                 block3:
;; @003d                               jump block1
;;
;;                                 block1:
;; @003d                               return
;; }
