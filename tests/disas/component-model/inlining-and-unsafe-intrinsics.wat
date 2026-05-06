;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[0]--function"
;;! flags = "-C inlining=y"
;;! unsafe_intrinsics = "unsafe-intrinsics"

(component
    (import "unsafe-intrinsics"
        (instance $intrinsics
            (export "store-data-address" (func (result u64)))
            (export "u8-native-load" (func (param "pointer" u64) (result u8)))
            (export "u8-native-store" (func (param "pointer" u64) (param "value" u8)))
        )
    )

    (core func $store-data-address' (canon lower (func $intrinsics "store-data-address")))
    (core func $u8-native-load' (canon lower (func $intrinsics "u8-native-load")))
    (core func $u8-native-store' (canon lower (func $intrinsics "u8-native-store")))

    (core module $m
        (import "" "store-data-address" (func $store-data-address (result i64)))
        (import "" "u8-native-load" (func $load (param i64) (result i32)))
        (import "" "u8-native-store" (func $store (param i64 i32)))
        (func (export "f")
            (local $x i32)
            (local.set $x (call $load (call $store-data-address)))
            (call $store (call $store-data-address)
                         (i32.add (local.get $x) (i32.const 1)))
        )
    )

    (core instance $i
        (instantiate $m
            (with "" (instance (export "store-data-address" (func $store-data-address'))
                               (export "u8-native-load" (func $u8-native-load'))
                               (export "u8-native-store" (func $u8-native-store'))))
        )
    )

    (func (export "f")
      (canon lift (core func $i "f"))
    )
)

;; function u0:0(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i64) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i64) -> i32 tail
;;     sig2 = (i64 vmctx, i64, i64, i32) tail
;;     fn0 = colocated u2147483648:0 sig0
;;     fn1 = colocated u2147483648:1 sig1
;;     fn2 = colocated u2147483648:2 sig2
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0153                               v5 = load.i64 notrap aligned readonly can_move vmctx v0+8
;; @0153                               v6 = load.i64 notrap aligned readonly can_move vmctx v5+104
;; @0155                               v9 = load.i8 notrap aligned v6
;;                                     v22 = iconst.i8 1
;;                                     v23 = iadd v9, v22  ; v22 = 1
;; @0160                               store notrap aligned v23, v6
;; @0162                               jump block1
;;
;;                                 block1:
;; @0162                               return
;; }
