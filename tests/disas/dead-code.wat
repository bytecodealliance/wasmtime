;;! target = "x86_64"

(module
  (func (param i32)
    (loop
      (block
        local.get 0
        br_if 0
        br 1)))

  (func (param i32)
    (loop
      (block
        br 1
        call $empty)))

  (func $empty)

  (func (param i32) (result i32)
    i32.const 1
    return
    i32.const 42)
)
;; function u0:0(i64 vmctx, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0023                               jump block2
;;
;;                                 block2:
;; @0029                               brif.i32 v2, block4, block5
;;
;;                                 block5:
;; @002b                               jump block2
;;
;;                                 block4:
;; @002e                               jump block3
;;
;;                                 block3:
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0032                               jump block2
;;
;;                                 block2:
;; @0036                               jump block2
;; }
;;
;; function u0:2(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @003f                               jump block1
;;
;;                                 block1:
;; @003f                               return
;; }
;;
;; function u0:3(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0042                               v4 = iconst.i32 1
;; @0044                               return v4  ; v4 = 1
;; }
