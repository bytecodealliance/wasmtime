;;! target = "x86_64"

(module
    (memory (export "mem") 1 1)
    (func (export "runif") (param $cond i32)
      i32.const 48
      (v128.load (i32.const 0))
      (v128.load (i32.const 16))
      (if (param v128) (param v128) (result v128 v128)
          (local.get $cond)
          (then i64x2.add
                (v128.load (i32.const 32)))
          (else i32x4.sub
                (v128.load (i32.const 0))))
      i16x8.mul
      v128.store)
)

;; function u0:0(i64 vmctx, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+88
;;     gv5 = load.i64 notrap aligned readonly checked gv3+80
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0036                               v3 = iconst.i32 48
;; @0038                               v4 = iconst.i32 0
;; @003a                               v5 = uextend.i64 v4  ; v4 = 0
;; @003a                               v6 = load.i64 notrap aligned readonly checked v0+80
;; @003a                               v7 = iadd v6, v5
;; @003a                               v8 = load.i8x16 little heap v7
;; @003e                               v9 = iconst.i32 16
;; @0040                               v10 = uextend.i64 v9  ; v9 = 16
;; @0040                               v11 = load.i64 notrap aligned readonly checked v0+80
;; @0040                               v12 = iadd v11, v10
;; @0040                               v13 = load.i8x16 little heap v12
;; @0046                               brif v2, block2, block4
;;
;;                                 block2:
;; @0048                               v16 = bitcast.i64x2 little v8
;; @0048                               v17 = bitcast.i64x2 little v13
;; @0048                               v18 = iadd v16, v17
;; @004b                               v19 = iconst.i32 32
;; @004d                               v20 = uextend.i64 v19  ; v19 = 32
;; @004d                               v21 = load.i64 notrap aligned readonly checked v0+80
;; @004d                               v22 = iadd v21, v20
;; @004d                               v23 = load.i8x16 little heap v22
;; @0051                               v26 = bitcast.i8x16 little v18
;; @0051                               jump block3(v26, v23)
;;
;;                                 block4:
;; @0052                               v27 = bitcast.i32x4 little v8
;; @0052                               v28 = bitcast.i32x4 little v13
;; @0052                               v29 = isub v27, v28
;; @0055                               v30 = iconst.i32 0
;; @0057                               v31 = uextend.i64 v30  ; v30 = 0
;; @0057                               v32 = load.i64 notrap aligned readonly checked v0+80
;; @0057                               v33 = iadd v32, v31
;; @0057                               v34 = load.i8x16 little heap v33
;; @005b                               v35 = bitcast.i8x16 little v29
;; @005b                               jump block3(v35, v34)
;;
;;                                 block3(v14: i8x16, v15: i8x16):
;; @005c                               v36 = bitcast.i16x8 little v14
;; @005c                               v37 = bitcast.i16x8 little v15
;; @005c                               v38 = imul v36, v37
;; @005f                               v39 = uextend.i64 v3  ; v3 = 48
;; @005f                               v40 = load.i64 notrap aligned readonly checked v0+80
;; @005f                               v41 = iadd v40, v39
;; @005f                               store little heap v38, v41
;; @0063                               jump block1
;;
;;                                 block1:
;; @0063                               return
;; }
