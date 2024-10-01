;;! target = "x86_64"

(module
  (memory 1)
  (func $main (local i32)
      (local.set 0 (i32.sub (i32.const 4) (i32.const 4)))
      (if
          (local.get 0)
          (then unreachable)
          (else (drop (i32.mul (i32.const 6) (local.get 0))))
       )
  )
  (start $main)
  (data (i32.const 0) "abcdefgh")
)

;; function u0:0(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @001f                               v2 = iconst.i32 0
;; @0021                               v3 = iconst.i32 4
;; @0023                               v4 = iconst.i32 4
;; @0025                               v5 = isub v3, v4  ; v3 = 4, v4 = 4
;; @002a                               brif v5, block2, block4
;;
;;                                 block2:
;; @002c                               trap user11
;;
;;                                 block4:
;; @002e                               v6 = iconst.i32 6
;; @0032                               v7 = imul v6, v5  ; v6 = 6
;; @0034                               jump block3
;;
;;                                 block3:
;; @0035                               jump block1
;;
;;                                 block1:
;; @0035                               return
;; }
