;;! target = "x86_64"

(module
  (func (param v128)
    (v128.store (i32.const 0) (i8x16.eq (local.get 0) (local.get 0))))
  (func (param v128)
    (v128.store (i32.const 0) (i16x8.eq (local.get 0) (local.get 0))))
  (func (param v128)
    (v128.store (i32.const 0) (i32x4.eq (local.get 0) (local.get 0))))
  (func (param v128)
    (v128.store (i32.const 0) (i64x2.eq (local.get 0) (local.get 0))))

  (func (param v128)
    (v128.store (i32.const 0) (i8x16.ne (local.get 0) (local.get 0))))
  (func (param v128)
    (v128.store (i32.const 0) (i16x8.ne (local.get 0) (local.get 0))))
  (func (param v128)
    (v128.store (i32.const 0) (i32x4.ne (local.get 0) (local.get 0))))
  (func (param v128)
    (v128.store (i32.const 0) (i64x2.ne (local.get 0) (local.get 0))))

  (func (param v128)
    (v128.store (i32.const 0) (i8x16.lt_s (local.get 0) (local.get 0))))
  (func (param v128)
    (v128.store (i32.const 0) (i16x8.lt_s (local.get 0) (local.get 0))))
  (func (param v128)
    (v128.store (i32.const 0) (i32x4.lt_s (local.get 0) (local.get 0))))
  (func (param v128)
    (v128.store (i32.const 0) (i64x2.lt_s (local.get 0) (local.get 0))))

  (func (param v128)
    (v128.store (i32.const 0) (i8x16.lt_u (local.get 0) (local.get 0))))
  (func (param v128)
    (v128.store (i32.const 0) (i16x8.lt_u (local.get 0) (local.get 0))))
  (func (param v128)
    (v128.store (i32.const 0) (i32x4.lt_u (local.get 0) (local.get 0))))

  (func (param v128)
    (v128.store (i32.const 0) (i8x16.gt_s (local.get 0) (local.get 0))))
  (func (param v128)
    (v128.store (i32.const 0) (i16x8.gt_s (local.get 0) (local.get 0))))
  (func (param v128)
    (v128.store (i32.const 0) (i32x4.gt_s (local.get 0) (local.get 0))))
  (func (param v128)
    (v128.store (i32.const 0) (i64x2.gt_s (local.get 0) (local.get 0))))

  (func (param v128)
    (v128.store (i32.const 0) (i8x16.gt_u (local.get 0) (local.get 0))))
  (func (param v128)
    (v128.store (i32.const 0) (i16x8.gt_u (local.get 0) (local.get 0))))
  (func (param v128)
    (v128.store (i32.const 0) (i32x4.gt_u (local.get 0) (local.get 0))))

  (func (param v128)
    (v128.store (i32.const 0) (f32x4.eq (local.get 0) (local.get 0))))
  (func (param v128)
    (v128.store (i32.const 0) (f64x2.eq (local.get 0) (local.get 0))))

  (func (param v128)
    (v128.store (i32.const 0) (f32x4.ne (local.get 0) (local.get 0))))
  (func (param v128)
    (v128.store (i32.const 0) (f64x2.ne (local.get 0) (local.get 0))))

  (func (param v128)
    (v128.store (i32.const 0) (f32x4.lt (local.get 0) (local.get 0))))
  (func (param v128)
    (v128.store (i32.const 0) (f64x2.lt (local.get 0) (local.get 0))))

  (func (param v128)
    (v128.store (i32.const 0) (f32x4.le (local.get 0) (local.get 0))))
  (func (param v128)
    (v128.store (i32.const 0) (f64x2.le (local.get 0) (local.get 0))))

  (func (param v128)
    (v128.store (i32.const 0) (f32x4.gt (local.get 0) (local.get 0))))
  (func (param v128)
    (v128.store (i32.const 0) (f64x2.gt (local.get 0) (local.get 0))))

  (func (param v128)
    (v128.store (i32.const 0) (f32x4.ge (local.get 0) (local.get 0))))
  (func (param v128)
    (v128.store (i32.const 0) (f64x2.ge (local.get 0) (local.get 0))))

  (memory 0)
)

;; function u0:0(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @003f                               v3 = iconst.i32 0
;; @0045                               v4 = icmp eq v2, v2
;; @0047                               v5 = uextend.i64 v3  ; v3 = 0
;; @0047                               v6 = global_value.i64 gv5
;; @0047                               v7 = iadd v6, v5
;; @0047                               store little heap v4, v7
;; @004b                               jump block1
;;
;;                                 block1:
;; @004b                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @004e                               v3 = iconst.i32 0
;; @0054                               v4 = bitcast.i16x8 little v2
;; @0054                               v5 = bitcast.i16x8 little v2
;; @0054                               v6 = icmp eq v4, v5
;; @0056                               v7 = uextend.i64 v3  ; v3 = 0
;; @0056                               v8 = global_value.i64 gv5
;; @0056                               v9 = iadd v8, v7
;; @0056                               store little heap v6, v9
;; @005a                               jump block1
;;
;;                                 block1:
;; @005a                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @005d                               v3 = iconst.i32 0
;; @0063                               v4 = bitcast.i32x4 little v2
;; @0063                               v5 = bitcast.i32x4 little v2
;; @0063                               v6 = icmp eq v4, v5
;; @0065                               v7 = uextend.i64 v3  ; v3 = 0
;; @0065                               v8 = global_value.i64 gv5
;; @0065                               v9 = iadd v8, v7
;; @0065                               store little heap v6, v9
;; @0069                               jump block1
;;
;;                                 block1:
;; @0069                               return
;; }
;;
;; function u0:3(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @006c                               v3 = iconst.i32 0
;; @0072                               v4 = bitcast.i64x2 little v2
;; @0072                               v5 = bitcast.i64x2 little v2
;; @0072                               v6 = icmp eq v4, v5
;; @0075                               v7 = uextend.i64 v3  ; v3 = 0
;; @0075                               v8 = global_value.i64 gv5
;; @0075                               v9 = iadd v8, v7
;; @0075                               store little heap v6, v9
;; @0079                               jump block1
;;
;;                                 block1:
;; @0079                               return
;; }
;;
;; function u0:4(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @007c                               v3 = iconst.i32 0
;; @0082                               v4 = icmp ne v2, v2
;; @0084                               v5 = uextend.i64 v3  ; v3 = 0
;; @0084                               v6 = global_value.i64 gv5
;; @0084                               v7 = iadd v6, v5
;; @0084                               store little heap v4, v7
;; @0088                               jump block1
;;
;;                                 block1:
;; @0088                               return
;; }
;;
;; function u0:5(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @008b                               v3 = iconst.i32 0
;; @0091                               v4 = bitcast.i16x8 little v2
;; @0091                               v5 = bitcast.i16x8 little v2
;; @0091                               v6 = icmp ne v4, v5
;; @0093                               v7 = uextend.i64 v3  ; v3 = 0
;; @0093                               v8 = global_value.i64 gv5
;; @0093                               v9 = iadd v8, v7
;; @0093                               store little heap v6, v9
;; @0097                               jump block1
;;
;;                                 block1:
;; @0097                               return
;; }
;;
;; function u0:6(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @009a                               v3 = iconst.i32 0
;; @00a0                               v4 = bitcast.i32x4 little v2
;; @00a0                               v5 = bitcast.i32x4 little v2
;; @00a0                               v6 = icmp ne v4, v5
;; @00a2                               v7 = uextend.i64 v3  ; v3 = 0
;; @00a2                               v8 = global_value.i64 gv5
;; @00a2                               v9 = iadd v8, v7
;; @00a2                               store little heap v6, v9
;; @00a6                               jump block1
;;
;;                                 block1:
;; @00a6                               return
;; }
;;
;; function u0:7(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @00a9                               v3 = iconst.i32 0
;; @00af                               v4 = bitcast.i64x2 little v2
;; @00af                               v5 = bitcast.i64x2 little v2
;; @00af                               v6 = icmp ne v4, v5
;; @00b2                               v7 = uextend.i64 v3  ; v3 = 0
;; @00b2                               v8 = global_value.i64 gv5
;; @00b2                               v9 = iadd v8, v7
;; @00b2                               store little heap v6, v9
;; @00b6                               jump block1
;;
;;                                 block1:
;; @00b6                               return
;; }
;;
;; function u0:8(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @00b9                               v3 = iconst.i32 0
;; @00bf                               v4 = icmp slt v2, v2
;; @00c1                               v5 = uextend.i64 v3  ; v3 = 0
;; @00c1                               v6 = global_value.i64 gv5
;; @00c1                               v7 = iadd v6, v5
;; @00c1                               store little heap v4, v7
;; @00c5                               jump block1
;;
;;                                 block1:
;; @00c5                               return
;; }
;;
;; function u0:9(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @00c8                               v3 = iconst.i32 0
;; @00ce                               v4 = bitcast.i16x8 little v2
;; @00ce                               v5 = bitcast.i16x8 little v2
;; @00ce                               v6 = icmp slt v4, v5
;; @00d0                               v7 = uextend.i64 v3  ; v3 = 0
;; @00d0                               v8 = global_value.i64 gv5
;; @00d0                               v9 = iadd v8, v7
;; @00d0                               store little heap v6, v9
;; @00d4                               jump block1
;;
;;                                 block1:
;; @00d4                               return
;; }
;;
;; function u0:10(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @00d7                               v3 = iconst.i32 0
;; @00dd                               v4 = bitcast.i32x4 little v2
;; @00dd                               v5 = bitcast.i32x4 little v2
;; @00dd                               v6 = icmp slt v4, v5
;; @00df                               v7 = uextend.i64 v3  ; v3 = 0
;; @00df                               v8 = global_value.i64 gv5
;; @00df                               v9 = iadd v8, v7
;; @00df                               store little heap v6, v9
;; @00e3                               jump block1
;;
;;                                 block1:
;; @00e3                               return
;; }
;;
;; function u0:11(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @00e6                               v3 = iconst.i32 0
;; @00ec                               v4 = bitcast.i64x2 little v2
;; @00ec                               v5 = bitcast.i64x2 little v2
;; @00ec                               v6 = icmp slt v4, v5
;; @00ef                               v7 = uextend.i64 v3  ; v3 = 0
;; @00ef                               v8 = global_value.i64 gv5
;; @00ef                               v9 = iadd v8, v7
;; @00ef                               store little heap v6, v9
;; @00f3                               jump block1
;;
;;                                 block1:
;; @00f3                               return
;; }
;;
;; function u0:12(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @00f6                               v3 = iconst.i32 0
;; @00fc                               v4 = icmp ult v2, v2
;; @00fe                               v5 = uextend.i64 v3  ; v3 = 0
;; @00fe                               v6 = global_value.i64 gv5
;; @00fe                               v7 = iadd v6, v5
;; @00fe                               store little heap v4, v7
;; @0102                               jump block1
;;
;;                                 block1:
;; @0102                               return
;; }
;;
;; function u0:13(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0105                               v3 = iconst.i32 0
;; @010b                               v4 = bitcast.i16x8 little v2
;; @010b                               v5 = bitcast.i16x8 little v2
;; @010b                               v6 = icmp ult v4, v5
;; @010d                               v7 = uextend.i64 v3  ; v3 = 0
;; @010d                               v8 = global_value.i64 gv5
;; @010d                               v9 = iadd v8, v7
;; @010d                               store little heap v6, v9
;; @0111                               jump block1
;;
;;                                 block1:
;; @0111                               return
;; }
;;
;; function u0:14(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0114                               v3 = iconst.i32 0
;; @011a                               v4 = bitcast.i32x4 little v2
;; @011a                               v5 = bitcast.i32x4 little v2
;; @011a                               v6 = icmp ult v4, v5
;; @011c                               v7 = uextend.i64 v3  ; v3 = 0
;; @011c                               v8 = global_value.i64 gv5
;; @011c                               v9 = iadd v8, v7
;; @011c                               store little heap v6, v9
;; @0120                               jump block1
;;
;;                                 block1:
;; @0120                               return
;; }
;;
;; function u0:15(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0123                               v3 = iconst.i32 0
;; @0129                               v4 = icmp sgt v2, v2
;; @012b                               v5 = uextend.i64 v3  ; v3 = 0
;; @012b                               v6 = global_value.i64 gv5
;; @012b                               v7 = iadd v6, v5
;; @012b                               store little heap v4, v7
;; @012f                               jump block1
;;
;;                                 block1:
;; @012f                               return
;; }
;;
;; function u0:16(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0132                               v3 = iconst.i32 0
;; @0138                               v4 = bitcast.i16x8 little v2
;; @0138                               v5 = bitcast.i16x8 little v2
;; @0138                               v6 = icmp sgt v4, v5
;; @013a                               v7 = uextend.i64 v3  ; v3 = 0
;; @013a                               v8 = global_value.i64 gv5
;; @013a                               v9 = iadd v8, v7
;; @013a                               store little heap v6, v9
;; @013e                               jump block1
;;
;;                                 block1:
;; @013e                               return
;; }
;;
;; function u0:17(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0141                               v3 = iconst.i32 0
;; @0147                               v4 = bitcast.i32x4 little v2
;; @0147                               v5 = bitcast.i32x4 little v2
;; @0147                               v6 = icmp sgt v4, v5
;; @0149                               v7 = uextend.i64 v3  ; v3 = 0
;; @0149                               v8 = global_value.i64 gv5
;; @0149                               v9 = iadd v8, v7
;; @0149                               store little heap v6, v9
;; @014d                               jump block1
;;
;;                                 block1:
;; @014d                               return
;; }
;;
;; function u0:18(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0150                               v3 = iconst.i32 0
;; @0156                               v4 = bitcast.i64x2 little v2
;; @0156                               v5 = bitcast.i64x2 little v2
;; @0156                               v6 = icmp sgt v4, v5
;; @0159                               v7 = uextend.i64 v3  ; v3 = 0
;; @0159                               v8 = global_value.i64 gv5
;; @0159                               v9 = iadd v8, v7
;; @0159                               store little heap v6, v9
;; @015d                               jump block1
;;
;;                                 block1:
;; @015d                               return
;; }
;;
;; function u0:19(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0160                               v3 = iconst.i32 0
;; @0166                               v4 = icmp ugt v2, v2
;; @0168                               v5 = uextend.i64 v3  ; v3 = 0
;; @0168                               v6 = global_value.i64 gv5
;; @0168                               v7 = iadd v6, v5
;; @0168                               store little heap v4, v7
;; @016c                               jump block1
;;
;;                                 block1:
;; @016c                               return
;; }
;;
;; function u0:20(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @016f                               v3 = iconst.i32 0
;; @0175                               v4 = bitcast.i16x8 little v2
;; @0175                               v5 = bitcast.i16x8 little v2
;; @0175                               v6 = icmp ugt v4, v5
;; @0177                               v7 = uextend.i64 v3  ; v3 = 0
;; @0177                               v8 = global_value.i64 gv5
;; @0177                               v9 = iadd v8, v7
;; @0177                               store little heap v6, v9
;; @017b                               jump block1
;;
;;                                 block1:
;; @017b                               return
;; }
;;
;; function u0:21(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @017e                               v3 = iconst.i32 0
;; @0184                               v4 = bitcast.i32x4 little v2
;; @0184                               v5 = bitcast.i32x4 little v2
;; @0184                               v6 = icmp ugt v4, v5
;; @0186                               v7 = uextend.i64 v3  ; v3 = 0
;; @0186                               v8 = global_value.i64 gv5
;; @0186                               v9 = iadd v8, v7
;; @0186                               store little heap v6, v9
;; @018a                               jump block1
;;
;;                                 block1:
;; @018a                               return
;; }
;;
;; function u0:22(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @018d                               v3 = iconst.i32 0
;; @0193                               v4 = bitcast.f32x4 little v2
;; @0193                               v5 = bitcast.f32x4 little v2
;; @0193                               v6 = fcmp eq v4, v5
;; @0195                               v7 = uextend.i64 v3  ; v3 = 0
;; @0195                               v8 = global_value.i64 gv5
;; @0195                               v9 = iadd v8, v7
;; @0195                               store little heap v6, v9
;; @0199                               jump block1
;;
;;                                 block1:
;; @0199                               return
;; }
;;
;; function u0:23(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @019c                               v3 = iconst.i32 0
;; @01a2                               v4 = bitcast.f64x2 little v2
;; @01a2                               v5 = bitcast.f64x2 little v2
;; @01a2                               v6 = fcmp eq v4, v5
;; @01a4                               v7 = uextend.i64 v3  ; v3 = 0
;; @01a4                               v8 = global_value.i64 gv5
;; @01a4                               v9 = iadd v8, v7
;; @01a4                               store little heap v6, v9
;; @01a8                               jump block1
;;
;;                                 block1:
;; @01a8                               return
;; }
;;
;; function u0:24(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @01ab                               v3 = iconst.i32 0
;; @01b1                               v4 = bitcast.f32x4 little v2
;; @01b1                               v5 = bitcast.f32x4 little v2
;; @01b1                               v6 = fcmp ne v4, v5
;; @01b3                               v7 = uextend.i64 v3  ; v3 = 0
;; @01b3                               v8 = global_value.i64 gv5
;; @01b3                               v9 = iadd v8, v7
;; @01b3                               store little heap v6, v9
;; @01b7                               jump block1
;;
;;                                 block1:
;; @01b7                               return
;; }
;;
;; function u0:25(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @01ba                               v3 = iconst.i32 0
;; @01c0                               v4 = bitcast.f64x2 little v2
;; @01c0                               v5 = bitcast.f64x2 little v2
;; @01c0                               v6 = fcmp ne v4, v5
;; @01c2                               v7 = uextend.i64 v3  ; v3 = 0
;; @01c2                               v8 = global_value.i64 gv5
;; @01c2                               v9 = iadd v8, v7
;; @01c2                               store little heap v6, v9
;; @01c6                               jump block1
;;
;;                                 block1:
;; @01c6                               return
;; }
;;
;; function u0:26(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @01c9                               v3 = iconst.i32 0
;; @01cf                               v4 = bitcast.f32x4 little v2
;; @01cf                               v5 = bitcast.f32x4 little v2
;; @01cf                               v6 = fcmp lt v4, v5
;; @01d1                               v7 = uextend.i64 v3  ; v3 = 0
;; @01d1                               v8 = global_value.i64 gv5
;; @01d1                               v9 = iadd v8, v7
;; @01d1                               store little heap v6, v9
;; @01d5                               jump block1
;;
;;                                 block1:
;; @01d5                               return
;; }
;;
;; function u0:27(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @01d8                               v3 = iconst.i32 0
;; @01de                               v4 = bitcast.f64x2 little v2
;; @01de                               v5 = bitcast.f64x2 little v2
;; @01de                               v6 = fcmp lt v4, v5
;; @01e0                               v7 = uextend.i64 v3  ; v3 = 0
;; @01e0                               v8 = global_value.i64 gv5
;; @01e0                               v9 = iadd v8, v7
;; @01e0                               store little heap v6, v9
;; @01e4                               jump block1
;;
;;                                 block1:
;; @01e4                               return
;; }
;;
;; function u0:28(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @01e7                               v3 = iconst.i32 0
;; @01ed                               v4 = bitcast.f32x4 little v2
;; @01ed                               v5 = bitcast.f32x4 little v2
;; @01ed                               v6 = fcmp le v4, v5
;; @01ef                               v7 = uextend.i64 v3  ; v3 = 0
;; @01ef                               v8 = global_value.i64 gv5
;; @01ef                               v9 = iadd v8, v7
;; @01ef                               store little heap v6, v9
;; @01f3                               jump block1
;;
;;                                 block1:
;; @01f3                               return
;; }
;;
;; function u0:29(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @01f6                               v3 = iconst.i32 0
;; @01fc                               v4 = bitcast.f64x2 little v2
;; @01fc                               v5 = bitcast.f64x2 little v2
;; @01fc                               v6 = fcmp le v4, v5
;; @01fe                               v7 = uextend.i64 v3  ; v3 = 0
;; @01fe                               v8 = global_value.i64 gv5
;; @01fe                               v9 = iadd v8, v7
;; @01fe                               store little heap v6, v9
;; @0202                               jump block1
;;
;;                                 block1:
;; @0202                               return
;; }
;;
;; function u0:30(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0205                               v3 = iconst.i32 0
;; @020b                               v4 = bitcast.f32x4 little v2
;; @020b                               v5 = bitcast.f32x4 little v2
;; @020b                               v6 = fcmp gt v4, v5
;; @020d                               v7 = uextend.i64 v3  ; v3 = 0
;; @020d                               v8 = global_value.i64 gv5
;; @020d                               v9 = iadd v8, v7
;; @020d                               store little heap v6, v9
;; @0211                               jump block1
;;
;;                                 block1:
;; @0211                               return
;; }
;;
;; function u0:31(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0214                               v3 = iconst.i32 0
;; @021a                               v4 = bitcast.f64x2 little v2
;; @021a                               v5 = bitcast.f64x2 little v2
;; @021a                               v6 = fcmp gt v4, v5
;; @021c                               v7 = uextend.i64 v3  ; v3 = 0
;; @021c                               v8 = global_value.i64 gv5
;; @021c                               v9 = iadd v8, v7
;; @021c                               store little heap v6, v9
;; @0220                               jump block1
;;
;;                                 block1:
;; @0220                               return
;; }
;;
;; function u0:32(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0223                               v3 = iconst.i32 0
;; @0229                               v4 = bitcast.f32x4 little v2
;; @0229                               v5 = bitcast.f32x4 little v2
;; @0229                               v6 = fcmp ge v4, v5
;; @022b                               v7 = uextend.i64 v3  ; v3 = 0
;; @022b                               v8 = global_value.i64 gv5
;; @022b                               v9 = iadd v8, v7
;; @022b                               store little heap v6, v9
;; @022f                               jump block1
;;
;;                                 block1:
;; @022f                               return
;; }
;;
;; function u0:33(i64 vmctx, i64, i8x16) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0232                               v3 = iconst.i32 0
;; @0238                               v4 = bitcast.f64x2 little v2
;; @0238                               v5 = bitcast.f64x2 little v2
;; @0238                               v6 = fcmp ge v4, v5
;; @023a                               v7 = uextend.i64 v3  ; v3 = 0
;; @023a                               v8 = global_value.i64 gv5
;; @023a                               v9 = iadd v8, v7
;; @023a                               store little heap v6, v9
;; @023e                               jump block1
;;
;;                                 block1:
;; @023e                               return
;; }
