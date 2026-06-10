;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "unsafe-intrinsics-wasm-call"
;;! unsafe_intrinsics = "unsafe-intrinsics"

;; Test the standalone (non-inlined) trampolines for the bounds-checked native
;; load/store intrinsics, with Spectre mitigations enabled (the default).

(component
    (import "unsafe-intrinsics"
        (instance $intrinsics
            (export "u32-checked-native-load"
                (func (param "base" u64) (param "offset" u64) (param "length" u64) (result u32)))
            (export "u32-checked-native-store"
                (func (param "base" u64) (param "offset" u64) (param "length" u64) (param "value" u32)))
        )
    )

    (core func $load' (canon lower (func $intrinsics "u32-checked-native-load")))
    (core func $store' (canon lower (func $intrinsics "u32-checked-native-store")))

    (core module $m
        (import "" "u32-checked-native-load" (func $load (param i64 i64 i64) (result i32)))
        (import "" "u32-checked-native-store" (func $store (param i64 i64 i64 i32)))
    )

    (core instance $i
        (instantiate $m
            (with "" (instance (export "u32-checked-native-load" (func $load'))
                               (export "u32-checked-native-store" (func $store'))))
        )
    )
)
;; function u0:0(i64 vmctx, i64, i64, i64, i64) -> i32 tail {
;; block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;;     v5 = iconst.i64 4
;;     v6, v7 = uadd_overflow v3, v5  ; v5 = 4
;;     v8 = icmp ugt v6, v4
;;     v9 = bor v7, v8
;;     v11 = iconst.i64 0
;;     v10 = iadd v2, v3
;;     v12 = select_spectre_guard v9, v11, v10  ; v11 = 0
;;     trapz v12, heap_oob
;;     v13 = load.i32 notrap aligned v12
;;     return v13
;; }
;;
;; function u0:0(i64 vmctx, i64, i64, i64, i64, i32) tail {
;; block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64, v5: i32):
;;     v6 = iconst.i64 4
;;     v7, v8 = uadd_overflow v3, v6  ; v6 = 4
;;     v9 = icmp ugt v7, v4
;;     v10 = bor v8, v9
;;     v12 = iconst.i64 0
;;     v11 = iadd v2, v3
;;     v13 = select_spectre_guard v10, v12, v11  ; v12 = 0
;;     trapz v13, heap_oob
;;     store notrap aligned v5, v13
;;     return
;; }
