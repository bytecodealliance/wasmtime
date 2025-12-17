;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "unsafe-intrinsics-wasm-call"
;;! unsafe_intrinsics = "unsafe-intrinsics"

;; Test that only the intrinsics that we actually use get compiled.

(component
    (import "unsafe-intrinsics"
        (instance $intrinsics
            (export "store-data-address" (func (result u64)))
            (export "u8-native-load" (func (param "pointer" u64) (result u8)))
        )
    )

    (core func $store-data-address' (canon lower (func $intrinsics "store-data-address")))
    (core func $u8-native-load' (canon lower (func $intrinsics "u8-native-load")))

    (core module $m
        (import "" "store-data-address" (func $store-data-address (result i64)))
        (import "" "u8-native-load" (func $load (param i64) (result i32)))

        ;; XXX: if we ever implement gc-sections/DCE during our linking, we will
        ;; need to update this to `ref.func` the imported functions or whatever
        ;; to ensure that they are rooted in that analysis and we do end up with
        ;; non-inlined function bodies for the intrinsics.
    )

    (core instance $i
        (instantiate $m
            (with "" (instance (export "store-data-address" (func $store-data-address'))
                               (export "u8-native-load" (func $u8-native-load'))))
        )
    )
)

;; function u0:0(i64 vmctx, i64) -> i64 tail {
;; block0(v0: i64, v1: i64):
;;     v2 = load.i64 notrap aligned readonly can_move vmctx v0+16
;;     v3 = load.i64 notrap aligned readonly can_move vmctx v2+104
;;     return v3
;; }
;;
;; function u0:0(i64 vmctx, i64, i64) -> i32 tail {
;; block0(v0: i64, v1: i64, v2: i64):
;;     v3 = load.i8 notrap aligned v2
;;     v4 = uextend.i32 v3
;;     return v4
;; }
