;;! target = "x86_64"

(module
  (func $main
    (v128.const i32x4 1 2 3 4)
    (v128.const i32x4 1 2 3 4)
    (call $add)
    drop
  )
  (func $add (param $a v128) (param $b v128) (result v128)
    (local.get $a)
    (local.get $b)
    (i32x4.add)
  )
  (start $main)
)

;; function u0:0(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     sig0 = (i64 vmctx, i64, i8x16, i8x16) -> i8x16 tail
;;     fn0 = colocated u0:1 sig0
;;     const0 = 0x00000004000000030000000200000001
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0021                               v2 = vconst.i8x16 const0
;; @0033                               v3 = vconst.i8x16 const0
;; @0045                               v4 = call fn0(v0, v0, v2, v3)  ; v2 = const0, v3 = const0
;; @0048                               jump block1
;;
;;                                 block1:
;; @0048                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i8x16, i8x16) -> i8x16 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16, v3: i8x16):
;; @004f                               v5 = bitcast.i32x4 little v2
;; @004f                               v6 = bitcast.i32x4 little v3
;; @004f                               v7 = iadd v5, v6
;; @0052                               v8 = bitcast.i8x16 little v7
;; @0052                               jump block1(v8)
;;
;;                                 block1(v4: i8x16):
;; @0052                               return v4
;; }
