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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 603979776 "VMMemoryDefinition+0x0"
;;     region3 = 603979784 "VMMemoryDefinition+0x8"
;;     region4 = 134217728 "PublicMemory"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0036                               v3 = iconst.i32 48
;; @0038                               v4 = iconst.i32 0
;; @003a                               v5 = uextend.i64 v4  ; v4 = 0
;; @003a                               v6 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @003a                               v7 = iadd v6, v5
;; @003a                               v8 = load.i8x16 little region4 v7
;; @003e                               v9 = iconst.i32 16
;; @0040                               v10 = uextend.i64 v9  ; v9 = 16
;; @0040                               v11 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @0040                               v12 = iadd v11, v10
;; @0040                               v13 = load.i8x16 little region4 v12
;; @0046                               brif v2, block2, block4
;;
;;                                 block2:
;; @0048                               v14 = bitcast.i64x2 little v8
;; @0048                               v15 = bitcast.i64x2 little v13
;; @0048                               v16 = iadd v14, v15
;; @004b                               v17 = iconst.i32 32
;; @004d                               v18 = uextend.i64 v17  ; v17 = 32
;; @004d                               v19 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @004d                               v20 = iadd v19, v18
;; @004d                               v21 = load.i8x16 little region4 v20
;; @0051                               v22 = bitcast.i8x16 little v16
;; @0051                               jump block3(v22, v21)
;;
;;                                 block4:
;; @0052                               v23 = bitcast.i32x4 little v8
;; @0052                               v24 = bitcast.i32x4 little v13
;; @0052                               v25 = isub v23, v24
;; @0055                               v26 = iconst.i32 0
;; @0057                               v27 = uextend.i64 v26  ; v26 = 0
;; @0057                               v28 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @0057                               v29 = iadd v28, v27
;; @0057                               v30 = load.i8x16 little region4 v29
;; @005b                               v31 = bitcast.i8x16 little v25
;; @005b                               jump block3(v31, v30)
;;
;;                                 block3(v32: i8x16, v33: i8x16):
;; @005c                               v34 = bitcast.i16x8 little v32
;; @005c                               v35 = bitcast.i16x8 little v33
;; @005c                               v36 = imul v34, v35
;; @005f                               v37 = uextend.i64 v3  ; v3 = 48
;; @005f                               v38 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @005f                               v39 = iadd v38, v37
;; @005f                               store little region4 v36, v39
;; @0063                               jump block1
;;
;;                                 block1:
;; @0063                               return
;; }
