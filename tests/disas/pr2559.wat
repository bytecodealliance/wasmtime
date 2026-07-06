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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) -> i8x16, i8x16, i8x16 tail
;;     fn0 = colocated u0:0 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @002e                               v2, v3, v4 = call fn0(v0, v0)
;; @0030                               v5 = iadd v3, v4
;; @0032                               v6, v7, v8 = call fn0(v0, v0)
;; @0034                               v9 = icmp uge v7, v8
;; @0036                               v10 = bitcast.i16x8 little v6
;; @0036                               v11 = bitcast.i16x8 little v9
;; @0036                               v12 = icmp ne v10, v11
;; @0038                               v13 = iconst.i32 13
;; @003a                               v14 = bitcast.i8x16 little v12
;; @003a                               brif v13, block1(v14), block2  ; v13 = 13
;;
;;                                 block2:
;; @003c                               v15 = iconst.i32 43
;; @003e                               v16 = bitcast.i8x16 little v12
;; @003e                               brif v15, block1(v16), block3  ; v15 = 43
;;
;;                                 block3:
;; @0040                               v17 = iconst.i32 13
;; @0042                               v18 = bitcast.i8x16 little v12
;; @0042                               brif v17, block1(v18), block4  ; v17 = 13
;;
;;                                 block4:
;; @0044                               v19 = iconst.i32 87
;; @0047                               v20 = bitcast.i8x16 little v12
;; @0047                               v21 = select.i8x16 v19, v5, v20  ; v19 = 87
;; @0048                               trap user12
;;
;;                                 block1(v24: i8x16):
;; @0055                               return v2, v5, v24
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i8x16, i8x16, i8x16 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) -> i8x16, i8x16, i8x16 tail
;;     fn0 = colocated u0:1 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0058                               v2, v3, v4 = call fn0(v0, v0)
;; @005a                               v5 = iadd v3, v4
;; @005c                               v6, v7, v8 = call fn0(v0, v0)
;; @005e                               v9 = icmp uge v7, v8
;; @0060                               v10 = bitcast.i16x8 little v6
;; @0060                               v11 = bitcast.i16x8 little v9
;; @0060                               v12 = icmp ne v10, v11
;; @0062                               v13 = iconst.i32 13
;; @0064                               v14 = bitcast.i8x16 little v12
;; @0064                               brif v13, block1(v14), block2  ; v13 = 13
;;
;;                                 block2:
;; @0066                               v15 = iconst.i32 43
;; @0068                               v16 = bitcast.i8x16 little v12
;; @0068                               brif v15, block1(v16), block3  ; v15 = 43
;;
;;                                 block3:
;; @006a                               v17 = iconst.i32 13
;; @006c                               v18 = bitcast.i8x16 little v12
;; @006c                               brif v17, block1(v18), block4  ; v17 = 13
;;
;;                                 block4:
;; @006e                               v19 = iconst.i32 87
;; @0071                               v20 = bitcast.i8x16 little v12
;; @0071                               v21 = select.i8x16 v19, v5, v20  ; v19 = 87
;; @0074                               trap user12
;;
;;                                 block1(v24: i8x16):
;; @0081                               return v2, v5, v24
;; }
