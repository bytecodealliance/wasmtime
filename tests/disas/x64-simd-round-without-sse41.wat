;;! target = "x86_64"

(module
  (func $f32x4.ceil (param v128) (result v128) (f32x4.ceil (local.get 0)))
  (func $f32x4.floor (param v128) (result v128) (f32x4.floor (local.get 0)))
  (func $f32x4.trunc (param v128) (result v128) (f32x4.trunc (local.get 0)))
  (func $f32x4.nearest (param v128) (result v128) (f32x4.nearest (local.get 0)))
  (func $f64x2.ceil (param v128) (result v128) (f64x2.ceil (local.get 0)))
  (func $f64x2.floor (param v128) (result v128) (f64x2.floor (local.get 0)))
  (func $f64x2.trunc (param v128) (result v128) (f64x2.trunc (local.get 0)))
  (func $f64x2.nearest (param v128) (result v128) (f64x2.nearest (local.get 0)))
  (func $i32x4.trunc_sat_f64x2_u_zero (param v128) (result v128) (i32x4.trunc_sat_f64x2_u_zero (local.get 0)))
)
;; function u0:0(i64 vmctx, i64, i8x16) -> i8x16 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, f32) -> f32 tail
;;     fn0 = colocated u1:38 sig0
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0023                               v4 = bitcast.f32x4 little v2
;; @0023                               v6 = vconst.f32x4 const0
;; @0023                               v7 = extractlane v4, 0
;; @0023                               v8 = call fn0(v0, v7)
;; @0023                               v9 = insertlane v6, v8, 0  ; v6 = const0
;; @0023                               v10 = extractlane v4, 1
;; @0023                               v11 = call fn0(v0, v10)
;; @0023                               v12 = insertlane v9, v11, 1
;; @0023                               v13 = extractlane v4, 2
;; @0023                               v14 = call fn0(v0, v13)
;; @0023                               v15 = insertlane v12, v14, 2
;; @0023                               v16 = extractlane v4, 3
;; @0023                               v17 = call fn0(v0, v16)
;; @0023                               v18 = insertlane v15, v17, 3
;; @0025                               v19 = bitcast.i8x16 little v18
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v19
;; }
;;
;; function u0:1(i64 vmctx, i64, i8x16) -> i8x16 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, f32) -> f32 tail
;;     fn0 = colocated u1:40 sig0
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @002a                               v4 = bitcast.f32x4 little v2
;; @002a                               v6 = vconst.f32x4 const0
;; @002a                               v7 = extractlane v4, 0
;; @002a                               v8 = call fn0(v0, v7)
;; @002a                               v9 = insertlane v6, v8, 0  ; v6 = const0
;; @002a                               v10 = extractlane v4, 1
;; @002a                               v11 = call fn0(v0, v10)
;; @002a                               v12 = insertlane v9, v11, 1
;; @002a                               v13 = extractlane v4, 2
;; @002a                               v14 = call fn0(v0, v13)
;; @002a                               v15 = insertlane v12, v14, 2
;; @002a                               v16 = extractlane v4, 3
;; @002a                               v17 = call fn0(v0, v16)
;; @002a                               v18 = insertlane v15, v17, 3
;; @002c                               v19 = bitcast.i8x16 little v18
;; @002c                               jump block1
;;
;;                                 block1:
;; @002c                               return v19
;; }
;;
;; function u0:2(i64 vmctx, i64, i8x16) -> i8x16 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, f32) -> f32 tail
;;     fn0 = colocated u1:42 sig0
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0031                               v4 = bitcast.f32x4 little v2
;; @0031                               v6 = vconst.f32x4 const0
;; @0031                               v7 = extractlane v4, 0
;; @0031                               v8 = call fn0(v0, v7)
;; @0031                               v9 = insertlane v6, v8, 0  ; v6 = const0
;; @0031                               v10 = extractlane v4, 1
;; @0031                               v11 = call fn0(v0, v10)
;; @0031                               v12 = insertlane v9, v11, 1
;; @0031                               v13 = extractlane v4, 2
;; @0031                               v14 = call fn0(v0, v13)
;; @0031                               v15 = insertlane v12, v14, 2
;; @0031                               v16 = extractlane v4, 3
;; @0031                               v17 = call fn0(v0, v16)
;; @0031                               v18 = insertlane v15, v17, 3
;; @0033                               v19 = bitcast.i8x16 little v18
;; @0033                               jump block1
;;
;;                                 block1:
;; @0033                               return v19
;; }
;;
;; function u0:3(i64 vmctx, i64, i8x16) -> i8x16 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, f32) -> f32 tail
;;     fn0 = colocated u1:44 sig0
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0038                               v4 = bitcast.f32x4 little v2
;; @0038                               v6 = vconst.f32x4 const0
;; @0038                               v7 = extractlane v4, 0
;; @0038                               v8 = call fn0(v0, v7)
;; @0038                               v9 = insertlane v6, v8, 0  ; v6 = const0
;; @0038                               v10 = extractlane v4, 1
;; @0038                               v11 = call fn0(v0, v10)
;; @0038                               v12 = insertlane v9, v11, 1
;; @0038                               v13 = extractlane v4, 2
;; @0038                               v14 = call fn0(v0, v13)
;; @0038                               v15 = insertlane v12, v14, 2
;; @0038                               v16 = extractlane v4, 3
;; @0038                               v17 = call fn0(v0, v16)
;; @0038                               v18 = insertlane v15, v17, 3
;; @003a                               v19 = bitcast.i8x16 little v18
;; @003a                               jump block1
;;
;;                                 block1:
;; @003a                               return v19
;; }
;;
;; function u0:4(i64 vmctx, i64, i8x16) -> i8x16 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, f64) -> f64 tail
;;     fn0 = colocated u1:39 sig0
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @003f                               v4 = bitcast.f64x2 little v2
;; @003f                               v6 = vconst.f64x2 const0
;; @003f                               v7 = extractlane v4, 0
;; @003f                               v8 = call fn0(v0, v7)
;; @003f                               v9 = insertlane v6, v8, 0  ; v6 = const0
;; @003f                               v10 = extractlane v4, 1
;; @003f                               v11 = call fn0(v0, v10)
;; @003f                               v12 = insertlane v9, v11, 1
;; @0041                               v13 = bitcast.i8x16 little v12
;; @0041                               jump block1
;;
;;                                 block1:
;; @0041                               return v13
;; }
;;
;; function u0:5(i64 vmctx, i64, i8x16) -> i8x16 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, f64) -> f64 tail
;;     fn0 = colocated u1:41 sig0
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0046                               v4 = bitcast.f64x2 little v2
;; @0046                               v6 = vconst.f64x2 const0
;; @0046                               v7 = extractlane v4, 0
;; @0046                               v8 = call fn0(v0, v7)
;; @0046                               v9 = insertlane v6, v8, 0  ; v6 = const0
;; @0046                               v10 = extractlane v4, 1
;; @0046                               v11 = call fn0(v0, v10)
;; @0046                               v12 = insertlane v9, v11, 1
;; @0048                               v13 = bitcast.i8x16 little v12
;; @0048                               jump block1
;;
;;                                 block1:
;; @0048                               return v13
;; }
;;
;; function u0:6(i64 vmctx, i64, i8x16) -> i8x16 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, f64) -> f64 tail
;;     fn0 = colocated u1:43 sig0
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @004d                               v4 = bitcast.f64x2 little v2
;; @004d                               v6 = vconst.f64x2 const0
;; @004d                               v7 = extractlane v4, 0
;; @004d                               v8 = call fn0(v0, v7)
;; @004d                               v9 = insertlane v6, v8, 0  ; v6 = const0
;; @004d                               v10 = extractlane v4, 1
;; @004d                               v11 = call fn0(v0, v10)
;; @004d                               v12 = insertlane v9, v11, 1
;; @004f                               v13 = bitcast.i8x16 little v12
;; @004f                               jump block1
;;
;;                                 block1:
;; @004f                               return v13
;; }
;;
;; function u0:7(i64 vmctx, i64, i8x16) -> i8x16 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, f64) -> f64 tail
;;     fn0 = colocated u1:45 sig0
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0054                               v4 = bitcast.f64x2 little v2
;; @0054                               v6 = vconst.f64x2 const0
;; @0054                               v7 = extractlane v4, 0
;; @0054                               v8 = call fn0(v0, v7)
;; @0054                               v9 = insertlane v6, v8, 0  ; v6 = const0
;; @0054                               v10 = extractlane v4, 1
;; @0054                               v11 = call fn0(v0, v10)
;; @0054                               v12 = insertlane v9, v11, 1
;; @0057                               v13 = bitcast.i8x16 little v12
;; @0057                               jump block1
;;
;;                                 block1:
;; @0057                               return v13
;; }
;;
;; function u0:8(i64 vmctx, i64, i8x16) -> i8x16 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @005c                               v4 = bitcast.f64x2 little v2
;; @005c                               v5 = extractlane v4, 0
;; @005c                               v6 = extractlane v4, 1
;; @005c                               v7 = fcvt_to_uint_sat.i32 v5
;; @005c                               v8 = fcvt_to_uint_sat.i32 v6
;; @005c                               v9 = vconst.i32x4 const0
;; @005c                               v10 = insertlane v9, v7, 0  ; v9 = const0
;; @005c                               v11 = insertlane v10, v8, 1
;; @005f                               v12 = bitcast.i8x16 little v11
;; @005f                               jump block1
;;
;;                                 block1:
;; @005f                               return v12
;; }
