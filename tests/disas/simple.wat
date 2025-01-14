;;! target = "x86_64"

(module
    (func $small1 (param i32) (result i32)
        (i32.add (local.get 0) (i32.const 1))
    )

    (func $small2 (param i32) (result i32)
        (return (i32.add (local.get 0) (i32.const 1)))
    )

    (func $infloop (result i32)
        (local i32)
        (loop (result i32)
            (i32.add (local.get 0) (i32.const 1))
            (local.set 0)
            (br 0)
        )
    )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0021                               v4 = iconst.i32 1
;; @0023                               v5 = iadd v2, v4  ; v4 = 1
;; @0024                               jump block1
;;
;;                                 block1:
;; @0024                               return v5
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0029                               v4 = iconst.i32 1
;; @002b                               v5 = iadd v2, v4  ; v4 = 1
;; @002c                               return v5
;; }
;;
;; function u0:2(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0030                               v3 = iconst.i32 0
;; @0032                               jump block2(v3)  ; v3 = 0
;;
;;                                 block2(v5: i32):
;; @0036                               v6 = iconst.i32 1
;; @0038                               v7 = iadd v5, v6  ; v6 = 1
;; @003b                               jump block2(v7)
;; }
