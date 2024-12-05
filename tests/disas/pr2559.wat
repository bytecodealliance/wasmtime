;;! target = "x86_64"

(module
  (type (;0;) (func (result v128 v128 v128)))

  (func $main1 (type 0) (result v128 v128 v128)
    call $main1
    i8x16.add
    call $main1
    i8x16.ge_u
    i16x8.ne
    i32.const 13
    br_if 0 (;@0;)
    i32.const 43
    br_if 0 (;@0;)
    i32.const 13
    br_if 0 (;@0;)
    i32.const 87
    select
    unreachable
    i32.const 0
    br_if 0 (;@0;)
    i32.const 13
    br_if 0 (;@0;)
    i32.const 43
    br_if 0 (;@0;)
  )
  (export "main1" (func $main1))

  (func $main2 (type 0) (result v128 v128 v128)
    call $main2
    i8x16.add
    call $main2
    i8x16.ge_u
    i16x8.ne
    i32.const 13
    br_if 0 (;@0;)
    i32.const 43
    br_if 0 (;@0;)
    i32.const 13
    br_if 0 (;@0;)
    i32.const 87
    select (result v128)
    unreachable
    i32.const 0
    br_if 0 (;@0;)
    i32.const 13
    br_if 0 (;@0;)
    i32.const 43
    br_if 0 (;@0;)
  )
  (export "main2" (func $main2))
)

;; function u0:0(i64 vmctx, i64) -> i8x16, i8x16, i8x16 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     sig0 = (i64 vmctx, i64) -> i8x16, i8x16, i8x16 tail
;;     fn0 = colocated u0:0 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @002e                               v5, v6, v7 = call fn0(v0, v0)
;; @0030                               v8 = iadd v6, v7
;; @0032                               v9, v10, v11 = call fn0(v0, v0)
;; @0034                               v12 = icmp uge v10, v11
;; @0036                               v13 = bitcast.i16x8 little v9
;; @0036                               v14 = bitcast.i16x8 little v12
;; @0036                               v15 = icmp ne v13, v14
;; @0038                               v16 = iconst.i32 13
;; @003a                               v17 = bitcast.i8x16 little v15
;; @003a                               brif v16, block1(v5, v8, v17), block2  ; v16 = 13
;;
;;                                 block2:
;; @003c                               v18 = iconst.i32 43
;; @003e                               v19 = bitcast.i8x16 little v15
;; @003e                               brif v18, block1(v5, v8, v19), block3  ; v18 = 43
;;
;;                                 block3:
;; @0040                               v20 = iconst.i32 13
;; @0042                               v21 = bitcast.i8x16 little v15
;; @0042                               brif v20, block1(v5, v8, v21), block4  ; v20 = 13
;;
;;                                 block4:
;; @0044                               v22 = iconst.i32 87
;; @0047                               v23 = bitcast.i8x16 little v15
;; @0047                               v24 = select.i8x16 v22, v8, v23  ; v22 = 87
;; @0048                               trap user11
;;
;;                                 block1(v2: i8x16, v3: i8x16, v4: i8x16):
;; @0055                               return v2, v3, v4
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i8x16, i8x16, i8x16 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     sig0 = (i64 vmctx, i64) -> i8x16, i8x16, i8x16 tail
;;     fn0 = colocated u0:1 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0058                               v5, v6, v7 = call fn0(v0, v0)
;; @005a                               v8 = iadd v6, v7
;; @005c                               v9, v10, v11 = call fn0(v0, v0)
;; @005e                               v12 = icmp uge v10, v11
;; @0060                               v13 = bitcast.i16x8 little v9
;; @0060                               v14 = bitcast.i16x8 little v12
;; @0060                               v15 = icmp ne v13, v14
;; @0062                               v16 = iconst.i32 13
;; @0064                               v17 = bitcast.i8x16 little v15
;; @0064                               brif v16, block1(v5, v8, v17), block2  ; v16 = 13
;;
;;                                 block2:
;; @0066                               v18 = iconst.i32 43
;; @0068                               v19 = bitcast.i8x16 little v15
;; @0068                               brif v18, block1(v5, v8, v19), block3  ; v18 = 43
;;
;;                                 block3:
;; @006a                               v20 = iconst.i32 13
;; @006c                               v21 = bitcast.i8x16 little v15
;; @006c                               brif v20, block1(v5, v8, v21), block4  ; v20 = 13
;;
;;                                 block4:
;; @006e                               v22 = iconst.i32 87
;; @0071                               v23 = bitcast.i8x16 little v15
;; @0071                               v24 = select.i8x16 v22, v8, v23  ; v22 = 87
;; @0074                               trap user11
;;
;;                                 block1(v2: i8x16, v3: i8x16, v4: i8x16):
;; @0081                               return v2, v3, v4
;; }
