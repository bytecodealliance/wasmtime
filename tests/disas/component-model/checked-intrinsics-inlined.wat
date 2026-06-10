;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[0]--function"
;;! flags = "-C inlining=y"
;;! unsafe_intrinsics = "unsafe-intrinsics"

;; Test the bounds-checked native load/store intrinsics, inlined into their
;; caller, with Spectre mitigations enabled (the default). The bounds check
;; should trap on out-of-bounds or overflow and additionally guard the computed
;; address with a `select_spectre_guard`.

(component
    (import "unsafe-intrinsics"
        (instance $intrinsics
            (export "store-data-address" (func (result u64)))
            (export "u32-checked-native-load"
                (func (param "base" u64) (param "offset" u64) (param "length" u64) (result u32)))
            (export "u32-checked-native-store"
                (func (param "base" u64) (param "offset" u64) (param "length" u64) (param "value" u32)))
        )
    )

    (core func $sda' (canon lower (func $intrinsics "store-data-address")))
    (core func $load' (canon lower (func $intrinsics "u32-checked-native-load")))
    (core func $store' (canon lower (func $intrinsics "u32-checked-native-store")))

    (core module $m
        (import "" "store-data-address" (func $sda (result i64)))
        (import "" "u32-checked-native-load" (func $load (param i64 i64 i64) (result i32)))
        (import "" "u32-checked-native-store" (func $store (param i64 i64 i64 i32)))
        (func (export "f") (param $offset i64) (param $length i64)
            (local $x i32)
            (local.set $x (call $load (call $sda) (local.get $offset) (local.get $length)))
            (call $store (call $sda) (local.get $offset) (local.get $length)
                         (i32.add (local.get $x) (i32.const 1)))
        )
    )

    (core instance $i
        (instantiate $m
            (with "" (instance (export "store-data-address" (func $sda'))
                               (export "u32-checked-native-load" (func $load'))
                               (export "u32-checked-native-store" (func $store'))))
        )
    )

    (func (export "f") (param "offset" u64) (param "length" u64)
      (canon lift (core func $i "f"))
    )
)
;; function u0:0(i64 vmctx, i64, i64, i64) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 72 "VMContext+0x48"
;;     region2 = 268435560 "VMStoreContext+0x68"
;;     region3 = 104 "VMContext+0x68"
;;     region4 = 136 "VMContext+0x88"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i64) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i64, i64, i64) -> i32 tail
;;     sig2 = (i64 vmctx, i64, i64, i64, i64, i32) tail
;;     fn0 = colocated u2147483648:0 sig0
;;     fn1 = colocated u2147483648:13 sig1
;;     fn2 = colocated u2147483648:14 sig2
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64):
;; @01b0                               v11 = iconst.i64 4
;; @01b0                               v12, v13 = uadd_overflow v2, v11  ; v11 = 4
;; @01b0                               v14 = icmp ugt v12, v3
;; @01b0                               v15 = bor v13, v14
;; @01b0                               v17 = iconst.i64 0
;; @01aa                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @01aa                               v8 = load.i64 notrap aligned readonly can_move region2 v7+104
;; @01b0                               v16 = iadd v8, v2
;; @01b0                               v18 = select_spectre_guard v15, v17, v16  ; v17 = 0
;; @01b0                               trapz v18, heap_oob
;; @01b0                               v19 = load.i32 notrap aligned v18
;; @01bf                               v29, v30 = uadd_overflow v2, v11  ; v11 = 4
;; @01bf                               v31 = icmp ugt v29, v3
;; @01bf                               v32 = bor v30, v31
;; @01bf                               v35 = select_spectre_guard v32, v17, v16  ; v17 = 0
;; @01bf                               trapz v35, heap_oob
;; @01bc                               v24 = iconst.i32 1
;; @01be                               v25 = iadd v19, v24  ; v24 = 1
;; @01bf                               store notrap aligned v25, v35
;; @01c1                               jump block1
;;
;;                                 block1:
;; @01c1                               return
;; }
