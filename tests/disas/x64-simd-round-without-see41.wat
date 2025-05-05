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
)
;; function u0:0(i64 vmctx, i64, i8x16) -> i8x16 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, f32) -> f32 tail
;;     fn0 = colocated u1:39 sig0
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0022                               v4 = bitcast.f32x4 little v2
;; @0022                               v6 = vconst.f32x4 const0
;; @0022                               v7 = extractlane v4, 0
;; @0022                               v8 = call fn0(v0, v7)
;; @0022                               v9 = insertlane v6, v8, 0  ; v6 = const0
;; @0022                               v10 = extractlane v4, 1
;; @0022                               v11 = call fn0(v0, v10)
;; @0022                               v12 = insertlane v9, v11, 1
;; @0022                               v13 = extractlane v4, 2
;; @0022                               v14 = call fn0(v0, v13)
;; @0022                               v15 = insertlane v12, v14, 2
;; @0022                               v16 = extractlane v4, 3
;; @0022                               v17 = call fn0(v0, v16)
;; @0022                               v18 = insertlane v15, v17, 3
;; @0024                               v19 = bitcast.i8x16 little v18
;; @0024                               jump block1
;;
;;                                 block1:
;; @0024                               return v19
;; }
;;
;; function u0:1(i64 vmctx, i64, i8x16) -> i8x16 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, f32) -> f32 tail
;;     fn0 = colocated u1:41 sig0
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0029                               v4 = bitcast.f32x4 little v2
;; @0029                               v6 = vconst.f32x4 const0
;; @0029                               v7 = extractlane v4, 0
;; @0029                               v8 = call fn0(v0, v7)
;; @0029                               v9 = insertlane v6, v8, 0  ; v6 = const0
;; @0029                               v10 = extractlane v4, 1
;; @0029                               v11 = call fn0(v0, v10)
;; @0029                               v12 = insertlane v9, v11, 1
;; @0029                               v13 = extractlane v4, 2
;; @0029                               v14 = call fn0(v0, v13)
;; @0029                               v15 = insertlane v12, v14, 2
;; @0029                               v16 = extractlane v4, 3
;; @0029                               v17 = call fn0(v0, v16)
;; @0029                               v18 = insertlane v15, v17, 3
;; @002b                               v19 = bitcast.i8x16 little v18
;; @002b                               jump block1
;;
;;                                 block1:
;; @002b                               return v19
;; }
;;
;; function u0:2(i64 vmctx, i64, i8x16) -> i8x16 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, f32) -> f32 tail
;;     fn0 = colocated u1:43 sig0
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0030                               v4 = bitcast.f32x4 little v2
;; @0030                               v6 = vconst.f32x4 const0
;; @0030                               v7 = extractlane v4, 0
;; @0030                               v8 = call fn0(v0, v7)
;; @0030                               v9 = insertlane v6, v8, 0  ; v6 = const0
;; @0030                               v10 = extractlane v4, 1
;; @0030                               v11 = call fn0(v0, v10)
;; @0030                               v12 = insertlane v9, v11, 1
;; @0030                               v13 = extractlane v4, 2
;; @0030                               v14 = call fn0(v0, v13)
;; @0030                               v15 = insertlane v12, v14, 2
;; @0030                               v16 = extractlane v4, 3
;; @0030                               v17 = call fn0(v0, v16)
;; @0030                               v18 = insertlane v15, v17, 3
;; @0032                               v19 = bitcast.i8x16 little v18
;; @0032                               jump block1
;;
;;                                 block1:
;; @0032                               return v19
;; }
;;
;; function u0:3(i64 vmctx, i64, i8x16) -> i8x16 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, f32) -> f32 tail
;;     fn0 = colocated u1:45 sig0
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0037                               v4 = bitcast.f32x4 little v2
;; @0037                               v6 = vconst.f32x4 const0
;; @0037                               v7 = extractlane v4, 0
;; @0037                               v8 = call fn0(v0, v7)
;; @0037                               v9 = insertlane v6, v8, 0  ; v6 = const0
;; @0037                               v10 = extractlane v4, 1
;; @0037                               v11 = call fn0(v0, v10)
;; @0037                               v12 = insertlane v9, v11, 1
;; @0037                               v13 = extractlane v4, 2
;; @0037                               v14 = call fn0(v0, v13)
;; @0037                               v15 = insertlane v12, v14, 2
;; @0037                               v16 = extractlane v4, 3
;; @0037                               v17 = call fn0(v0, v16)
;; @0037                               v18 = insertlane v15, v17, 3
;; @0039                               v19 = bitcast.i8x16 little v18
;; @0039                               jump block1
;;
;;                                 block1:
;; @0039                               return v19
;; }
;;
;; function u0:4(i64 vmctx, i64, i8x16) -> i8x16 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, f64) -> f64 tail
;;     fn0 = colocated u1:40 sig0
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @003e                               v4 = bitcast.f64x2 little v2
;; @003e                               v6 = vconst.f64x2 const0
;; @003e                               v7 = extractlane v4, 0
;; @003e                               v8 = call fn0(v0, v7)
;; @003e                               v9 = insertlane v6, v8, 0  ; v6 = const0
;; @003e                               v10 = extractlane v4, 1
;; @003e                               v11 = call fn0(v0, v10)
;; @003e                               v12 = insertlane v9, v11, 1
;; @0040                               v13 = bitcast.i8x16 little v12
;; @0040                               jump block1
;;
;;                                 block1:
;; @0040                               return v13
;; }
;;
;; function u0:5(i64 vmctx, i64, i8x16) -> i8x16 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, f64) -> f64 tail
;;     fn0 = colocated u1:42 sig0
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0045                               v4 = bitcast.f64x2 little v2
;; @0045                               v6 = vconst.f64x2 const0
;; @0045                               v7 = extractlane v4, 0
;; @0045                               v8 = call fn0(v0, v7)
;; @0045                               v9 = insertlane v6, v8, 0  ; v6 = const0
;; @0045                               v10 = extractlane v4, 1
;; @0045                               v11 = call fn0(v0, v10)
;; @0045                               v12 = insertlane v9, v11, 1
;; @0047                               v13 = bitcast.i8x16 little v12
;; @0047                               jump block1
;;
;;                                 block1:
;; @0047                               return v13
;; }
;;
;; function u0:6(i64 vmctx, i64, i8x16) -> i8x16 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, f64) -> f64 tail
;;     fn0 = colocated u1:44 sig0
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @004c                               v4 = bitcast.f64x2 little v2
;; @004c                               v6 = vconst.f64x2 const0
;; @004c                               v7 = extractlane v4, 0
;; @004c                               v8 = call fn0(v0, v7)
;; @004c                               v9 = insertlane v6, v8, 0  ; v6 = const0
;; @004c                               v10 = extractlane v4, 1
;; @004c                               v11 = call fn0(v0, v10)
;; @004c                               v12 = insertlane v9, v11, 1
;; @004e                               v13 = bitcast.i8x16 little v12
;; @004e                               jump block1
;;
;;                                 block1:
;; @004e                               return v13
;; }
;;
;; function u0:7(i64 vmctx, i64, i8x16) -> i8x16 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, f64) -> f64 tail
;;     fn0 = colocated u1:46 sig0
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0053                               v4 = bitcast.f64x2 little v2
;; @0053                               v6 = vconst.f64x2 const0
;; @0053                               v7 = extractlane v4, 0
;; @0053                               v8 = call fn0(v0, v7)
;; @0053                               v9 = insertlane v6, v8, 0  ; v6 = const0
;; @0053                               v10 = extractlane v4, 1
;; @0053                               v11 = call fn0(v0, v10)
;; @0053                               v12 = insertlane v9, v11, 1
;; @0056                               v13 = bitcast.i8x16 little v12
;; @0056                               jump block1
;;
;;                                 block1:
;; @0056                               return v13
;; }
