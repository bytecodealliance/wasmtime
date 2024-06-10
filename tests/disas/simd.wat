;;! target = "x86_64"

(module
  (func $test_splat (result i32)
    i32.const 42
    i32x4.splat
    i32x4.extract_lane 0
  )

  (func $test_insert_lane (result i32)
      v128.const i64x2 0 0
      i32.const 99
      i32x4.replace_lane 1
      i32x4.extract_lane 1
  )

  (func $test_const (result i32)
    v128.const i32x4 1 2 3 4
    i32x4.extract_lane 3
  )

  (func $test_locals (local i32 v128)
    local.get 0
    i32x4.splat
    local.set 1
  )

  (export "test_splat" (func $test_splat))
  (export "test_insert_lane" (func $test_insert_lane))
  (export "test_const" (func $test_const))
)

;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @004e                               v3 = iconst.i32 42
;; @0050                               v4 = splat.i32x4 v3  ; v3 = 42
;; @0052                               v5 = extractlane v4, 0
;; @0055                               jump block1(v5)
;;
;;                                 block1(v2: i32):
;; @0055                               return v2
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0058                               v3 = vconst.i8x16 const0
;; @006a                               v4 = iconst.i32 99
;; @006d                               v5 = bitcast.i32x4 little v3  ; v3 = const0
;; @006d                               v6 = insertlane v5, v4, 1  ; v4 = 99
;; @0070                               v7 = extractlane v6, 1
;; @0073                               jump block1(v7)
;;
;;                                 block1(v2: i32):
;; @0073                               return v2
;; }
;;
;; function u0:2(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     const0 = 0x00000004000000030000000200000001
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0076                               v3 = vconst.i8x16 const0
;; @0088                               v4 = bitcast.i32x4 little v3  ; v3 = const0
;; @0088                               v5 = extractlane v4, 3
;; @008b                               jump block1(v5)
;;
;;                                 block1(v2: i32):
;; @008b                               return v2
;; }
;;
;; function u0:3(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @008e                               v2 = iconst.i32 0
;; @0090                               v3 = vconst.i8x16 const0
;; @0094                               v4 = splat.i32x4 v2  ; v2 = 0
;; @0096                               v5 = bitcast.i8x16 little v4
;; @0098                               jump block1
;;
;;                                 block1:
;; @0098                               return
;; }
