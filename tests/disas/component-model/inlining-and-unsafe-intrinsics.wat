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
;; @0153                               jump block2
;;
;;                                 block2:
;;                                     jump block3
;;
;;                                 block3:
;; @0155                               jump block4
;;
;;                                 block4:
;; @0153                               v4 = load.i64 notrap aligned readonly can_move v0+64
;;                                     v17 = load.i64 notrap aligned readonly can_move vmctx v4+16
;;                                     v18 = load.i64 notrap aligned readonly can_move vmctx v17+104
;;                                     v20 = load.i8 notrap aligned v18
;;                                     jump block5
;;
;;                                 block5:
;; @0159                               jump block6
;;
;;                                 block6:
;;                                     jump block7
;;
;;                                 block7:
;; @0160                               jump block8
;;
;;                                 block8:
;;                                     v28 = iconst.i8 1
;;                                     v29 = iadd.i8 v20, v28  ; v28 = 1
;;                                     store notrap aligned v29, v18
;;                                     jump block9
;;
;;                                 block9:
;; @0162                               jump block1
;;
;;                                 block1:
;; @0162                               return
;; }
