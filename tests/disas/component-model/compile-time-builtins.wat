;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[1]--function"
;;! flags = "-C inlining=y -Wconcurrency-support=y"
;;! unsafe_intrinsics = "unsafe-intrinsics"

(component
    (import "unsafe-intrinsics"
         (instance $intrinsics
             (export "store-data-address" (func (result u64)))
             (export "u64-native-load" (func (param "pointer" u64) (result u64)))
             (export "u8-native-load" (func (param "pointer" u64) (result u8)))
             (export "u8-native-store" (func (param "pointer" u64) (param "value" u8)))
         )
    )

    (component $HostBuf
        (import "unsafe-intrinsics"
            (instance $intrinsics
                (export "store-data-address" (func (result u64)))
                (export "u64-native-load" (func (param "pointer" u64) (result u64)))
                (export "u8-native-load" (func (param "pointer" u64) (result u8)))
                (export "u8-native-store" (func (param "pointer" u64) (param "value" u8)))
            )
        )

        ;; The core Wasm module that implements the safe API.
        (core module $host-buf-impl
            (import "" "store-data-address" (func $store-data-address (result i64)))
            (import "" "u64-native-load" (func $u64-native-load (param i64) (result i64)))
            (import "" "u8-native-load" (func $u8-native-load (param i64) (result i32)))
            (import "" "u8-native-store" (func $u8-native-store (param i64 i32)))

            ;; Load the `ExposedBuf::buf_ptr` field
            (func $get-buf-ptr (result i64)
                (call $u64-native-load (i64.add (call $store-data-address) (i64.const 0)))
            )

            ;; Load the `ExposedBuf::buf_len` field
            (func $get-buf-len (result i64)
                (call $u64-native-load (i64.add (call $store-data-address) (i64.const 8)))
            )

            ;; Check that `$i` is within `ExposedBuf` buffer's bounds, raising a trap
            ;; otherwise.
            (func $bounds-check (param $i i64)
                (if (i64.lt_u (local.get $i) (call $get-buf-len))
                    (then (return))
                    (else (unreachable))
                )
            )

            ;; A safe function to get the `i`th byte from `ExposedBuf`'s buffer,
            ;; raising a trap on out-of-bounds accesses.
            (func (export "get") (param $i i64) (result i32)
                (call $bounds-check (local.get $i))
                (call $u8-native-load (i64.add (call $get-buf-ptr) (local.get $i)))
            )

            ;; A safe function to set the `i`th byte in `ExposedBuf`'s buffer,
            ;; raising a trap on out-of-bounds accesses.
            (func (export "set") (param $i i64) (param $value i32)
                (call $bounds-check (local.get $i))
                (call $u8-native-store (i64.add (call $get-buf-ptr) (local.get $i))
                                       (local.get $value))
            )

            ;; A safe function to get the length of the `ExposedBuf` buffer.
            (func (export "len") (result i64)
                (call $get-buf-len)
            )
        )

        ;; Lower the imported intrinsics from component functions to core functions.
        (core func $store-data-address' (canon lower (func $intrinsics "store-data-address")))
        (core func $u64-native-load' (canon lower (func $intrinsics "u64-native-load")))
        (core func $u8-native-load' (canon lower (func $intrinsics "u8-native-load")))
        (core func $u8-native-store' (canon lower (func $intrinsics "u8-native-store")))

        ;; Instantiate our safe API implementation, passing in the lowered unsafe
        ;; intrinsics as its imports.
        (core instance $instance
            (instantiate $host-buf-impl
                (with "" (instance
                    (export "store-data-address" (func $store-data-address'))
                    (export "u64-native-load" (func $u64-native-load'))
                    (export "u8-native-load" (func $u8-native-load'))
                    (export "u8-native-store" (func $u8-native-store'))
                ))
            )
        )

        ;; Lift the safe API's exports from core functions to component functions
        ;; and export them.
        (func (export "get") (param "i" u64) (result u8)
            (canon lift (core func $instance "get"))
        )
        (func (export "set") (param "i" u64) (param "value" u8)
            (canon lift (core func $instance "set"))
        )
        (func (export "len") (result u64)
            (canon lift (core func $instance "len"))
        )
    )
    (instance $host-buf (instantiate $HostBuf (with "unsafe-intrinsics" (instance $intrinsics))))

    (component $Guest
        ;; Import the safe API.
        (import "host-buf"
            (instance $host-buf
                (export "get" (func (param "i" u64) (result u8)))
                (export "set" (func (param "i" u64) (param "value" u8)))
                (export "len" (func (result u64)))
            )
        )

        ;; Define this component's core module implementation.
        (core module $main-impl
            (import "" "get" (func $get (param i64) (result i32)))
            (import "" "set" (func $set (param i64 i32)))
            (import "" "len" (func $len (result i64)))

            (func (export "main")
                (local $i i64)
                (local $n i64)

                (local.set $i (i64.const 0))
                (local.set $n (call $len))

                (loop $loop
                    ;; When we have iterated over every byte in the
                    ;; buffer, exit.
                    (if (i64.ge_u (local.get $i) (local.get $n))
                        (then (return)))

                    ;; Increment the `i`th byte in the buffer.
                    (call $set (local.get $i)
                               (i32.add (call $get (local.get $i))
                                        (i32.const 1)))

                    ;; Increment `i` and continue to the next iteration
                    ;; of the loop.
                    (local.set $i (i64.add (local.get $i) (i64.const 1)))
                    (br $loop)
                )
            )
        )

        ;; Lower the imported safe APIs from component functions to core functions.
        (core func $get' (canon lower (func $host-buf "get")))
        (core func $set' (canon lower (func $host-buf "set")))
        (core func $len' (canon lower (func $host-buf "len")))

        ;; Instantiate our module, providing the lowered safe APIs as imports.
        (core instance $instance
            (instantiate $main-impl
                (with "" (instance
                    (export "get" (func $get'))
                    (export "set" (func $set'))
                    (export "len" (func $len'))
                ))
            )
        )

        ;; Lift the implementation's `main` from a core function to a component function
        ;; and export it!
        (func (export "main")
            (canon lift (core func $instance "main"))
        )
    )
    (instance $guest (instantiate $Guest (with "host-buf" (instance $host-buf))))

    (export "main" (func $guest "main"))
)

;; function u1:0(i64 vmctx, i64) tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 25 ""
;;     region3 = 44 ""
;;     region4 = 78 ""
;;     region5 = 227 ""
;;     region6 = 175 ""
;;     region7 = 113 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned region1 gv4+24
;;     gv6 = vmctx
;;     gv7 = load.i64 notrap aligned readonly can_move region0 gv6+8
;;     gv8 = load.i64 notrap aligned region1 gv7+24
;;     gv9 = vmctx
;;     gv10 = load.i64 notrap aligned readonly can_move region0 gv9+8
;;     gv11 = load.i64 notrap aligned region1 gv10+24
;;     gv12 = vmctx
;;     gv13 = load.i64 notrap aligned readonly can_move region0 gv12+8
;;     gv14 = load.i64 notrap aligned region1 gv13+24
;;     gv15 = vmctx
;;     gv16 = load.i64 notrap aligned readonly can_move region0 gv15+8
;;     gv17 = load.i64 notrap aligned region1 gv16+24
;;     gv18 = vmctx
;;     gv19 = load.i64 notrap aligned readonly can_move region0 gv18+8
;;     gv20 = load.i64 notrap aligned region1 gv19+24
;;     gv21 = vmctx
;;     gv22 = load.i64 notrap aligned readonly can_move region0 gv21+8
;;     gv23 = load.i64 notrap aligned region1 gv22+24
;;     gv24 = vmctx
;;     gv25 = load.i64 notrap aligned readonly can_move region0 gv24+8
;;     gv26 = load.i64 notrap aligned region1 gv25+24
;;     gv27 = vmctx
;;     gv28 = load.i64 notrap aligned readonly can_move region0 gv27+8
;;     gv29 = load.i64 notrap aligned region1 gv28+24
;;     gv30 = vmctx
;;     gv31 = load.i64 notrap aligned readonly can_move region0 gv30+8
;;     gv32 = load.i64 notrap aligned region1 gv31+24
;;     gv33 = vmctx
;;     gv34 = load.i64 notrap aligned readonly can_move region0 gv33+8
;;     gv35 = load.i64 notrap aligned region1 gv34+24
;;     gv36 = vmctx
;;     gv37 = load.i64 notrap aligned readonly can_move region0 gv36+8
;;     gv38 = load.i64 notrap aligned region1 gv37+24
;;     gv39 = vmctx
;;     gv40 = load.i64 notrap aligned readonly can_move region0 gv39+8
;;     gv41 = load.i64 notrap aligned region1 gv40+24
;;     sig0 = (i64 vmctx, i64) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i64) -> i32 tail
;;     sig2 = (i64 vmctx, i64, i64, i32) tail
;;     sig3 = (i64 vmctx, i64) tail
;;     sig4 = (i64 vmctx, i64) -> i64 tail
;;     sig5 = (i64 vmctx, i64) -> i64 tail
;;     sig6 = (i64 vmctx, i64) -> i64 tail
;;     sig7 = (i64 vmctx, i64, i64) -> i64 tail
;;     sig8 = (i64 vmctx, i64) tail
;;     sig9 = (i64 vmctx, i64, i64) -> i32 tail
;;     sig10 = (i64 vmctx, i64, i64) tail
;;     sig11 = (i64 vmctx, i64) -> i64 tail
;;     sig12 = (i64 vmctx, i64, i64) -> i32 tail
;;     sig13 = (i64 vmctx, i64) -> i64 tail
;;     sig14 = (i64 vmctx, i64) -> i64 tail
;;     sig15 = (i64 vmctx, i64, i64) -> i64 tail
;;     sig16 = (i64 vmctx, i64) -> i64 tail
;;     sig17 = (i64 vmctx, i64, i64) -> i64 tail
;;     sig18 = (i64 vmctx, i64) tail
;;     sig19 = (i64 vmctx, i64, i64, i32) tail
;;     sig20 = (i64 vmctx, i64, i64) tail
;;     sig21 = (i64 vmctx, i64) -> i64 tail
;;     sig22 = (i64 vmctx, i64, i64, i32) tail
;;     sig23 = (i64 vmctx, i64) -> i64 tail
;;     sig24 = (i64 vmctx, i64) -> i64 tail
;;     sig25 = (i64 vmctx, i64, i64) -> i64 tail
;;     sig26 = (i64 vmctx, i64) -> i64 tail
;;     sig27 = (i64 vmctx, i64, i64) -> i64 tail
;;     fn0 = colocated u2:2 sig0
;;     fn1 = colocated u2:0 sig1
;;     fn2 = colocated u2:1 sig2
;;     fn3 = colocated u0:5 sig4
;;     fn4 = colocated u0:1 sig5
;;     fn5 = colocated u2147483648:0 sig6
;;     fn6 = colocated u2147483648:7 sig7
;;     fn7 = colocated u0:3 sig9
;;     fn8 = colocated u0:2 sig10
;;     fn9 = colocated u0:0 sig11
;;     fn10 = colocated u2147483648:1 sig12
;;     fn11 = colocated u0:1 sig13
;;     fn12 = colocated u2147483648:0 sig14
;;     fn13 = colocated u2147483648:7 sig15
;;     fn14 = colocated u2147483648:0 sig16
;;     fn15 = colocated u2147483648:7 sig17
;;     fn16 = colocated u0:4 sig19
;;     fn17 = colocated u0:2 sig20
;;     fn18 = colocated u0:0 sig21
;;     fn19 = colocated u2147483648:2 sig22
;;     fn20 = colocated u0:1 sig23
;;     fn21 = colocated u2147483648:0 sig24
;;     fn22 = colocated u2147483648:7 sig25
;;     fn23 = colocated u2147483648:0 sig26
;;     fn24 = colocated u2147483648:7 sig27
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0579                               jump block6
;;
;;                                 block6:
;;                                     jump block10
;;
;;                                 block10:
;; @0579                               v4 = load.i64 notrap aligned readonly can_move region2 v0+72
;;                                     v23 = load.i64 notrap aligned readonly can_move region3 v4+232
;;                                     v24 = load.i32 notrap aligned region4 v23
;;                                     trapz v24, user26
;;                                     jump block13
;;
;;                                 block13:
;;                                     v25 = load.i64 notrap aligned readonly can_move region3 v4+208
;;                                     v26 = load.i32 notrap aligned region5 v25
;;                                     jump block16
;;
;;                                 block16:
;;                                     jump block18
;;
;;                                 block18:
;;                                     v30 = load.i64 notrap aligned readonly can_move region2 v4+72
;;                                     v32 = load.i64 notrap aligned readonly can_move region0 v30+8
;;                                     v33 = load.i64 notrap aligned readonly can_move region6 v32+104
;;                                     v34 = iconst.i64 8
;;                                     v35 = iadd v33, v34  ; v34 = 8
;;                                     v37 = load.i64 notrap aligned region7 v35
;;                                     jump block19
;;
;;                                 block19:
;;                                     jump block20
;;
;;                                 block20:
;;                                     jump block17
;;
;;                                 block17:
;;                                     jump block15
;;
;;                                 block15:
;;                                     jump block11
;;
;;                                 block11:
;;                                     jump block8
;;
;;                                 block8:
;;                                     jump block7
;;
;;                                 block7:
;;                                     jump block21
;;
;;                                 block21:
;; @0573                               v2 = iconst.i64 0
;;                                     v127 = iconst.i8 1
;; @0595                               v15 = iconst.i64 1
;; @057d                               jump block2(v2)  ; v2 = 0
;;
;;                                 block2(v6: i64):
;; @0583                               v8 = icmp uge v6, v37
;; @0584                               brif v8, block4, block5
;;
;;                                 block4:
;; @0586                               return
;;
;;                                 block5:
;; @058c                               jump block22
;;
;;                                 block22:
;;                                     jump block26
;;
;;                                 block26:
;;                                     v49 = load.i32 notrap aligned region4 v23
;;                                     trapz v49, user26
;;                                     jump block29
;;
;;                                 block29:
;;                                     v51 = load.i32 notrap aligned region5 v25
;;                                     jump block32
;;
;;                                 block32:
;;                                     jump block34
;;
;;                                 block34:
;;                                     jump block39
;;
;;                                 block39:
;;                                     v131 = iadd.i64 v33, v34  ; v34 = 8
;;                                     v62 = load.i64 notrap aligned region7 v131
;;                                     jump block40
;;
;;                                 block40:
;;                                     jump block41
;;
;;                                 block41:
;;                                     v63 = icmp.i64 ult v6, v62
;;                                     trapz v63, user12
;;                                     jump block36
;;
;;                                 block36:
;;                                     jump block42
;;
;;                                 block42:
;;                                     jump block43
;;
;;                                 block43:
;;                                     v71 = load.i64 notrap aligned region7 v33
;;                                     jump block44
;;
;;                                 block44:
;;                                     jump block45
;;
;;                                 block45:
;;                                     v72 = iadd.i64 v71, v6
;;                                     v74 = load.i8 notrap aligned region7 v72
;;                                     jump block33
;;
;;                                 block33:
;;                                     jump block31
;;
;;                                 block31:
;;                                     jump block27
;;
;;                                 block27:
;;                                     jump block24
;;
;;                                 block24:
;;                                     jump block23
;;
;;                                 block23:
;;                                     jump block46
;;
;;                                 block46:
;; @0591                               jump block47
;;
;;                                 block47:
;;                                     jump block51
;;
;;                                 block51:
;;                                     jump block54
;;
;;                                 block54:
;;                                     jump block57
;;
;;                                 block57:
;;                                     jump block59
;;
;;                                 block59:
;;                                     jump block64
;;
;;                                 block64:
;;                                     jump block65
;;
;;                                 block65:
;;                                     jump block66
;;
;;                                 block66:
;;                                     jump block61
;;
;;                                 block61:
;;                                     jump block67
;;
;;                                 block67:
;;                                     jump block68
;;
;;                                 block68:
;;                                     jump block69
;;
;;                                 block69:
;;                                     jump block70
;;
;;                                 block70:
;;                                     v132 = iconst.i8 1
;;                                     v133 = iadd.i8 v74, v132  ; v132 = 1
;;                                     store notrap aligned region7 v133, v72
;;                                     jump block58
;;
;;                                 block58:
;;                                     jump block56
;;
;;                                 block56:
;;                                     jump block52
;;
;;                                 block52:
;;                                     jump block49
;;
;;                                 block49:
;;                                     jump block48
;;
;;                                 block48:
;;                                     jump block71
;;
;;                                 block71:
;;                                     v134 = iconst.i64 1
;;                                     v135 = iadd.i64 v6, v134  ; v134 = 1
;; @059a                               jump block2(v135)
;; }
