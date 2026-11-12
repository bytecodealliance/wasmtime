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

    ;; A host buffer. Its rep is an index into the store data's
    ;; `ExposedVec<ExposedBuf>`.
    (type $host-buf (resource (rep i32)))
    (core func $host-buf-new (canon resource.new $host-buf))

    ;; The core Wasm module that implements the safe API.
    ;;
    ;; This component defines the `host-buf` resource, so lifted `borrow`s
    ;; of it arrive as reps rather than handles.
    (core module $host-buf-impl
        (import "" "store-data-address" (func $store-data-address (result i64)))
        (import "" "u64-native-load" (func $u64-native-load (param i64) (result i64)))
        (import "" "u8-native-load" (func $u8-native-load (param i64) (result i32)))
        (import "" "u8-native-store" (func $u8-native-store (param i64 i32)))
        (import "" "host-buf-new" (func $host-buf-new (param i32) (result i32)))

        ;; Load the `ExposedVec::ptr` field
        (func $get-vec-ptr (result i64)
            (call $u64-native-load (i64.add (call $store-data-address) (i64.const 0)))
        )

        ;; Load the `ExposedVec::len` field
        (func $get-vec-len (result i64)
            (call $u64-native-load (i64.add (call $store-data-address) (i64.const 8)))
        )

        ;; Check that `$index` is within the `ExposedVec`'s bounds, raising
        ;; a trap otherwise.
        (func $vec-bounds-check (param $index i32)
            (if (i64.lt_u (i64.extend_i32_u (local.get $index)) (call $get-vec-len))
                (then (return))
                (else (unreachable))
            )
        )

        ;; Get the address of the `$buf`th `ExposedBuf` in the
        ;; `ExposedVec`, raising a trap on out-of-bounds accesses.
        (func $get-buf-addr (param $buf i32) (result i64)
            (call $vec-bounds-check (local.get $buf))
            (i64.add (call $get-vec-ptr)
                     ;; `size_of::<ExposedBuf>() == 16`
                     (i64.mul (i64.extend_i32_u (local.get $buf)) (i64.const 16)))
        )

        ;; Load the `$buf`th `ExposedBuf::buf_ptr` field
        (func $get-buf-ptr (param $buf i32) (result i64)
            (call $u64-native-load (i64.add (call $get-buf-addr (local.get $buf))
                                            (i64.const 0)))
        )

        ;; Load the `$buf`th `ExposedBuf::buf_len` field
        (func $get-buf-len (param $buf i32) (result i64)
            (call $u64-native-load (i64.add (call $get-buf-addr (local.get $buf))
                                            (i64.const 8)))
        )

        ;; Check that `$i` is within the `$buf`th `ExposedBuf` buffer's
        ;; bounds, raising a trap otherwise.
        (func $buf-bounds-check (param $buf i32) (param $i i64)
            (if (i64.lt_u (local.get $i) (call $get-buf-len (local.get $buf)))
                (then (return))
                (else (unreachable))
            )
        )

        ;; A safe function to create a `host-buf` resource for the
        ;; `$index`th `ExposedBuf`, raising a trap on out-of-bounds
        ;; indices.
        (func (export "new") (param $index i32) (result i32)
            (call $vec-bounds-check (local.get $index))
            (call $host-buf-new (local.get $index))
        )

        ;; A safe function to get the `i`th byte from an `ExposedBuf`'s
        ;; buffer, raising a trap on out-of-bounds accesses.
        (func (export "get") (param $buf i32) (param $i i64) (result i32)
            (call $buf-bounds-check (local.get $buf) (local.get $i))
            (call $u8-native-load (i64.add (call $get-buf-ptr (local.get $buf))
                                           (local.get $i)))
        )

        ;; A safe function to set the `i`th byte in an `ExposedBuf`'s
        ;; buffer, raising a trap on out-of-bounds accesses.
        (func (export "set") (param $buf i32) (param $i i64) (param $value i32)
            (call $buf-bounds-check (local.get $buf) (local.get $i))
            (call $u8-native-store (i64.add (call $get-buf-ptr (local.get $buf))
                                            (local.get $i))
                                   (local.get $value))
        )

        ;; A safe function to get the length of an `ExposedBuf`'s buffer.
        (func (export "len") (param $buf i32) (result i64)
            (call $get-buf-len (local.get $buf))
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
                (export "host-buf-new" (func $host-buf-new))
            ))
        )
    )

    ;; Export the resource type and lift the safe API's exports from core
    ;; functions to component functions and export them.
    (export $host-buf' "host-buf" (type $host-buf))
    (func (export "new") (param "index" u32) (result (own $host-buf'))
        (canon lift (core func $instance "new"))
    )
    (func (export "get") (param "buf" (borrow $host-buf')) (param "i" u64) (result u8)
        (canon lift (core func $instance "get"))
    )
    (func (export "set") (param "buf" (borrow $host-buf')) (param "i" u64) (param "value" u8)
        (canon lift (core func $instance "set"))
    )
    (func (export "len") (param "buf" (borrow $host-buf')) (result u64)
        (canon lift (core func $instance "len"))
    )
)
    (instance $host-buf (instantiate $HostBuf (with "unsafe-intrinsics" (instance $intrinsics))))

    (component $Guest
        ;; Import the safe API.
        (import "host-buf"
            (instance $host-buf
                (export "host-buf" (type $host-buf (sub resource)))
                (export "get" (func (param "buf" (borrow $host-buf)) (param "i" u64) (result u8)))
                (export "set" (func (param "buf" (borrow $host-buf)) (param "i" u64) (param "value" u8)))
                (export "len" (func (param "buf" (borrow $host-buf)) (result u64)))
            )
        )
        (alias export $host-buf "host-buf" (type $host-buf))

        ;; Define this component's core module implementation.
        (core module $main-impl
            (import "" "drop" (func $drop (param i32)))
            (import "" "get" (func $get (param i32 i64) (result i32)))
            (import "" "set" (func $set (param i32 i64 i32)))
            (import "" "len" (func $len (param i32) (result i64)))

            (func (export "main") (param $buf i32)
                (local $i i64)
                (local $n i64)

                (local.set $i (i64.const 0))
                (local.set $n (call $len (local.get $buf)))

                (loop $loop
                    ;; When we have iterated over every byte in the
                    ;; buffer, exit.
                    (if (i64.ge_u (local.get $i) (local.get $n))
                        (then
                            ;; Borrows must be dropped before returning.
                            (call $drop (local.get $buf))
                            (return)))

                    ;; Increment the `i`th byte in the buffer.
                    (call $set (local.get $buf)
                               (local.get $i)
                               (i32.add (call $get (local.get $buf) (local.get $i))
                                        (i32.const 1)))

                    ;; Increment `i` and continue to the next iteration
                    ;; of the loop.
                    (local.set $i (i64.add (local.get $i) (i64.const 1)))
                    (br $loop)
                )
            )
        )

        ;; Lower the imported safe APIs from component functions to core functions.
        (core func $drop (canon resource.drop $host-buf))
        (core func $get' (canon lower (func $host-buf "get")))
        (core func $set' (canon lower (func $host-buf "set")))
        (core func $len' (canon lower (func $host-buf "len")))

        ;; Instantiate our module, providing the lowered safe APIs as imports.
        (core instance $instance
            (instantiate $main-impl
                (with "" (instance
                    (export "drop" (func $drop))
                    (export "get" (func $get'))
                    (export "set" (func $set'))
                    (export "len" (func $len'))
                ))
            )
        )

        ;; Export the host buffer resource type, so that the host can name
        ;; it, along with a lifted version of the implementation's `main` core
        ;; function.
        (export $host-buf' "host-buf" (type $host-buf))
        (func (export "main") (param "buf" (borrow $host-buf'))
            (canon lift (core func $instance "main"))
        )
    )
    (instance $guest (instantiate $Guest (with "host-buf" (instance $host-buf))))

    (export $host-buf "host-buf" (type $guest "host-buf"))
    (export "main" (func $guest "main") (func (param "buf" (borrow $host-buf))))
)

;; function u1:0(i64 vmctx, i64, i32) tail {
;;     ss0 = explicit_slot 32, align = 8
;;     ss1 = explicit_slot 32, align = 8
;;     ss2 = explicit_slot 32, align = 8
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 42 ""
;;     region3 = 174 ""
;;     region4 = 57 ""
;;     region5 = 209 ""
;;     region6 = 68 ""
;;     region7 = 237 ""
;;     region8 = 126 ""
;;     region9 = 221 ""
;;     region10 = 112 ""
;;     region11 = 71 ""
;;     region12 = 149 ""
;;     region13 = 93 ""
;;     region14 = 30 ""
;;     region15 = 184 ""
;;     region16 = 108 ""
;;     region17 = 8 ""
;;     region18 = 218 ""
;;     region19 = 252 ""
;;     region20 = 13 ""
;;     region21 = 34 ""
;;     region22 = 195 ""
;;     region23 = 158 ""
;;     region24 = 175 ""
;;     region25 = 113 ""
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
;;     gv42 = vmctx
;;     gv43 = load.i64 notrap aligned readonly can_move region0 gv42+8
;;     gv44 = load.i64 notrap aligned region1 gv43+24
;;     gv45 = vmctx
;;     gv46 = load.i64 notrap aligned readonly can_move region0 gv45+8
;;     gv47 = load.i64 notrap aligned region1 gv46+24
;;     gv48 = vmctx
;;     gv49 = load.i64 notrap aligned readonly can_move region0 gv48+8
;;     gv50 = load.i64 notrap aligned region1 gv49+24
;;     gv51 = vmctx
;;     gv52 = load.i64 notrap aligned readonly can_move region0 gv51+8
;;     gv53 = load.i64 notrap aligned region1 gv52+24
;;     gv54 = vmctx
;;     gv55 = load.i64 notrap aligned readonly can_move region0 gv54+8
;;     gv56 = load.i64 notrap aligned region1 gv55+24
;;     gv57 = vmctx
;;     gv58 = load.i64 notrap aligned readonly can_move region0 gv57+8
;;     gv59 = load.i64 notrap aligned region1 gv58+24
;;     gv60 = vmctx
;;     gv61 = load.i64 notrap aligned readonly can_move region0 gv60+8
;;     gv62 = load.i64 notrap aligned region1 gv61+24
;;     gv63 = vmctx
;;     gv64 = load.i64 notrap aligned readonly can_move region0 gv63+8
;;     gv65 = load.i64 notrap aligned region1 gv64+24
;;     gv66 = vmctx
;;     gv67 = load.i64 notrap aligned readonly can_move region0 gv66+8
;;     gv68 = load.i64 notrap aligned region1 gv67+24
;;     gv69 = vmctx
;;     gv70 = load.i64 notrap aligned readonly can_move region0 gv69+8
;;     gv71 = load.i64 notrap aligned region1 gv70+24
;;     gv72 = vmctx
;;     gv73 = load.i64 notrap aligned readonly can_move region0 gv72+8
;;     gv74 = load.i64 notrap aligned region1 gv73+24
;;     gv75 = vmctx
;;     gv76 = load.i64 notrap aligned readonly can_move region0 gv75+8
;;     gv77 = load.i64 notrap aligned region1 gv76+24
;;     gv78 = vmctx
;;     gv79 = load.i64 notrap aligned readonly can_move region0 gv78+8
;;     gv80 = load.i64 notrap aligned region1 gv79+24
;;     gv81 = vmctx
;;     gv82 = load.i64 notrap aligned readonly can_move region0 gv81+8
;;     gv83 = load.i64 notrap aligned region1 gv82+24
;;     gv84 = vmctx
;;     gv85 = load.i64 notrap aligned readonly can_move region0 gv84+8
;;     gv86 = load.i64 notrap aligned region1 gv85+24
;;     gv87 = vmctx
;;     gv88 = load.i64 notrap aligned readonly can_move region0 gv87+8
;;     gv89 = load.i64 notrap aligned region1 gv88+24
;;     gv90 = vmctx
;;     gv91 = load.i64 notrap aligned readonly can_move region0 gv90+8
;;     gv92 = load.i64 notrap aligned region1 gv91+24
;;     gv93 = vmctx
;;     gv94 = load.i64 notrap aligned readonly can_move region0 gv93+8
;;     gv95 = load.i64 notrap aligned region1 gv94+24
;;     gv96 = vmctx
;;     gv97 = load.i64 notrap aligned readonly can_move region0 gv96+8
;;     gv98 = load.i64 notrap aligned region1 gv97+24
;;     gv99 = vmctx
;;     gv100 = load.i64 notrap aligned readonly can_move region0 gv99+8
;;     gv101 = load.i64 notrap aligned region1 gv100+24
;;     sig0 = (i64 vmctx, i64, i32) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i32) tail
;;     sig2 = (i64 vmctx, i64, i32, i64) -> i32 tail
;;     sig3 = (i64 vmctx, i64, i32, i64, i32) tail
;;     sig4 = (i64 vmctx, i64) tail
;;     sig5 = (i64 vmctx, i64, i32, i32) tail
;;     sig6 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig7 = (i64 vmctx, i64, i32) -> i64 tail
;;     sig8 = (i64 vmctx, i64, i32) -> i64 tail
;;     sig9 = (i64 vmctx, i64, i32) -> i64 tail
;;     sig10 = (i64 vmctx, i64, i64) -> i64 tail
;;     sig11 = (i64 vmctx, i64, i32) tail
;;     sig12 = (i64 vmctx, i64) -> i64 tail
;;     sig13 = (i64 vmctx, i64) -> i64 tail
;;     sig14 = (i64 vmctx, i64) -> i64 tail
;;     sig15 = (i64 vmctx, i64, i64) -> i64 tail
;;     sig16 = (i64 vmctx, i64) -> i64 tail
;;     sig17 = (i64 vmctx, i64, i64) -> i64 tail
;;     sig18 = (i64 vmctx, i64) tail
;;     sig19 = (i64 vmctx, i64, i32, i32) tail
;;     sig20 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig21 = (i64 vmctx, i64, i32, i64) -> i32 tail
;;     sig22 = (i64 vmctx, i64, i32, i64) tail
;;     sig23 = (i64 vmctx, i64, i32) -> i64 tail
;;     sig24 = (i64 vmctx, i64, i64) -> i32 tail
;;     sig25 = (i64 vmctx, i64, i32) -> i64 tail
;;     sig26 = (i64 vmctx, i64, i32) -> i64 tail
;;     sig27 = (i64 vmctx, i64, i64) -> i64 tail
;;     sig28 = (i64 vmctx, i64, i32) tail
;;     sig29 = (i64 vmctx, i64) -> i64 tail
;;     sig30 = (i64 vmctx, i64) -> i64 tail
;;     sig31 = (i64 vmctx, i64) -> i64 tail
;;     sig32 = (i64 vmctx, i64, i64) -> i64 tail
;;     sig33 = (i64 vmctx, i64) -> i64 tail
;;     sig34 = (i64 vmctx, i64, i64) -> i64 tail
;;     sig35 = (i64 vmctx, i64, i32) -> i64 tail
;;     sig36 = (i64 vmctx, i64, i64) -> i64 tail
;;     sig37 = (i64 vmctx, i64, i32) tail
;;     sig38 = (i64 vmctx, i64) -> i64 tail
;;     sig39 = (i64 vmctx, i64) -> i64 tail
;;     sig40 = (i64 vmctx, i64) -> i64 tail
;;     sig41 = (i64 vmctx, i64, i64) -> i64 tail
;;     sig42 = (i64 vmctx, i64) -> i64 tail
;;     sig43 = (i64 vmctx, i64, i64) -> i64 tail
;;     sig44 = (i64 vmctx, i64) tail
;;     sig45 = (i64 vmctx, i64, i32, i32) tail
;;     sig46 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig47 = (i64 vmctx, i64, i32, i64, i32) tail
;;     sig48 = (i64 vmctx, i64, i32, i64) tail
;;     sig49 = (i64 vmctx, i64, i32) -> i64 tail
;;     sig50 = (i64 vmctx, i64, i64, i32) tail
;;     sig51 = (i64 vmctx, i64, i32) -> i64 tail
;;     sig52 = (i64 vmctx, i64, i32) -> i64 tail
;;     sig53 = (i64 vmctx, i64, i64) -> i64 tail
;;     sig54 = (i64 vmctx, i64, i32) tail
;;     sig55 = (i64 vmctx, i64) -> i64 tail
;;     sig56 = (i64 vmctx, i64) -> i64 tail
;;     sig57 = (i64 vmctx, i64) -> i64 tail
;;     sig58 = (i64 vmctx, i64, i64) -> i64 tail
;;     sig59 = (i64 vmctx, i64) -> i64 tail
;;     sig60 = (i64 vmctx, i64, i64) -> i64 tail
;;     sig61 = (i64 vmctx, i64, i32) -> i64 tail
;;     sig62 = (i64 vmctx, i64, i64) -> i64 tail
;;     sig63 = (i64 vmctx, i64, i32) tail
;;     sig64 = (i64 vmctx, i64) -> i64 tail
;;     sig65 = (i64 vmctx, i64) -> i64 tail
;;     sig66 = (i64 vmctx, i64) -> i64 tail
;;     sig67 = (i64 vmctx, i64, i64) -> i64 tail
;;     sig68 = (i64 vmctx, i64) -> i64 tail
;;     sig69 = (i64 vmctx, i64, i64) -> i64 tail
;;     fn0 = colocated u2:2 sig0
;;     fn1 = colocated u2:0 sig2
;;     fn2 = colocated u2:1 sig3
;;     fn3 = colocated u0:10 sig7
;;     fn4 = colocated u0:5 sig8
;;     fn5 = colocated u0:3 sig9
;;     fn6 = colocated u2147483648:7 sig10
;;     fn7 = colocated u0:2 sig11
;;     fn8 = colocated u0:0 sig12
;;     fn9 = colocated u0:1 sig13
;;     fn10 = colocated u2147483648:0 sig14
;;     fn11 = colocated u2147483648:7 sig15
;;     fn12 = colocated u2147483648:0 sig16
;;     fn13 = colocated u2147483648:7 sig17
;;     fn14 = colocated u0:8 sig21
;;     fn15 = colocated u0:6 sig22
;;     fn16 = colocated u0:4 sig23
;;     fn17 = colocated u2147483648:1 sig24
;;     fn18 = colocated u0:5 sig25
;;     fn19 = colocated u0:3 sig26
;;     fn20 = colocated u2147483648:7 sig27
;;     fn21 = colocated u0:2 sig28
;;     fn22 = colocated u0:0 sig29
;;     fn23 = colocated u0:1 sig30
;;     fn24 = colocated u2147483648:0 sig31
;;     fn25 = colocated u2147483648:7 sig32
;;     fn26 = colocated u2147483648:0 sig33
;;     fn27 = colocated u2147483648:7 sig34
;;     fn28 = colocated u0:3 sig35
;;     fn29 = colocated u2147483648:7 sig36
;;     fn30 = colocated u0:2 sig37
;;     fn31 = colocated u0:0 sig38
;;     fn32 = colocated u0:1 sig39
;;     fn33 = colocated u2147483648:0 sig40
;;     fn34 = colocated u2147483648:7 sig41
;;     fn35 = colocated u2147483648:0 sig42
;;     fn36 = colocated u2147483648:7 sig43
;;     fn37 = colocated u0:9 sig47
;;     fn38 = colocated u0:6 sig48
;;     fn39 = colocated u0:4 sig49
;;     fn40 = colocated u2147483648:2 sig50
;;     fn41 = colocated u0:5 sig51
;;     fn42 = colocated u0:3 sig52
;;     fn43 = colocated u2147483648:7 sig53
;;     fn44 = colocated u0:2 sig54
;;     fn45 = colocated u0:0 sig55
;;     fn46 = colocated u0:1 sig56
;;     fn47 = colocated u2147483648:0 sig57
;;     fn48 = colocated u2147483648:7 sig58
;;     fn49 = colocated u2147483648:0 sig59
;;     fn50 = colocated u2147483648:7 sig60
;;     fn51 = colocated u0:3 sig61
;;     fn52 = colocated u2147483648:7 sig62
;;     fn53 = colocated u0:2 sig63
;;     fn54 = colocated u0:0 sig64
;;     fn55 = colocated u0:1 sig65
;;     fn56 = colocated u2147483648:0 sig66
;;     fn57 = colocated u2147483648:7 sig67
;;     fn58 = colocated u2147483648:0 sig68
;;     fn59 = colocated u2147483648:7 sig69
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @075b                               jump block6
;;
;;                                 block6:
;;                                     jump block10
;;
;;                                 block10:
;; @075b                               v5 = load.i64 notrap aligned readonly can_move region2 v0+104
;;                                     v31 = load.i64 notrap aligned readonly can_move region4 v5+328
;;                                     v32 = load.i32 notrap aligned region5 v31
;;                                     trapz v32, user26
;;                                     jump block13
;;
;;                                 block13:
;;                                     v37 = load.i64 notrap aligned readonly can_move region0 v5+8
;;                                     v38 = load.i64 notrap aligned region6 v37+136
;;                                     v36 = stack_addr.i64 ss0
;;                                     store notrap aligned region7 v38, v36
;;                                     v29 = iconst.i32 0
;;                                     store notrap aligned region8 v29, v36+8  ; v29 = 0
;;                                     v34 = iconst.i32 1
;;                                     store notrap aligned region9 v34, v36+12  ; v34 = 1
;;                                     v39 = load.i32 notrap aligned region10 v37+128
;;                                     store notrap aligned region11 v39, v36+16
;;                                     store notrap aligned region10 v29, v37+128  ; v29 = 0
;;                                     v41 = load.i32 notrap aligned region12 v37+132
;;                                     store notrap aligned region13 v41, v36+20
;;                                     store notrap aligned region12 v29, v37+132  ; v29 = 0
;;                                     v43 = load.i64 notrap aligned region14 v37+152
;;                                     store notrap aligned region15 v43, v36+24
;;                                     store notrap aligned region6 v36, v37+136
;;                                     jump block17
;;
;;                                 block17:
;;                                     v50 = iconst.i64 8
;;                                     v51 = icmp.i64 ult v43, v50  ; v50 = 8
;;                                     brif v51, block18, block15
;;
;;                                 block18:
;;                                     v115 = load.i64 notrap aligned readonly can_move region2 v5+104
;;                                     v55 = load.i64 notrap aligned region16 v115+120
;;                                     v52 = iconst.i32 -1
;;                                     v53 = iadd.i32 v2, v52  ; v52 = -1
;;                                     v54 = uextend.i64 v53
;;                                     v56 = icmp ult v54, v55
;;                                     brif v56, block19, block15
;;
;;                                 block19:
;;                                     v57 = load.i64 notrap aligned region17 v115+112
;;                                     v58 = iconst.i64 24
;;                                     v59 = imul.i64 v54, v58  ; v58 = 24
;;                                     v60 = iadd v57, v59
;;                                     v61 = load.i32 notrap aligned region18 v60
;;                                     v401 = iconst.i32 -1
;;                                     v402 = iadd v61, v401  ; v401 = -1
;;                                     v403 = iconst.i32 1
;;                                     v404 = icmp ule v402, v403  ; v403 = 1
;;                                     brif v404, block20, block15
;;
;;                                 block20:
;;                                     v66 = load.i32 notrap aligned region19 v60+4
;;                                     v67 = iconst.i32 2
;;                                     v68 = icmp eq v66, v67  ; v67 = 2
;;                                     brif v68, block21, block15
;;
;;                                 block21:
;;                                     v69 = load.i32 notrap aligned region20 v60+8
;;                                     v405 = iconst.i32 2
;;                                     v70 = load.i64 notrap aligned readonly can_move region21 v37+144
;;                                     v376 = iconst.i64 3
;;                                     v377 = ishl.i64 v43, v376  ; v376 = 3
;;                                     v73 = iadd v70, v377
;;                                     store notrap aligned region22 v405, v73  ; v405 = 2
;;                                     store.i32 notrap aligned region23 v2, v73+4
;;                                     v75 = iconst.i64 1
;;                                     v76 = iadd.i64 v43, v75  ; v75 = 1
;;                                     store notrap aligned region14 v76, v37+152
;;                                     jump block16(v69)
;;
;;                                 block15 cold:
;;                                     v113 = load.i64 notrap aligned readonly can_move region3 v5+152
;;                                     v406 = load.i64 notrap aligned readonly can_move region2 v5+104
;;                                     v114 = call_indirect sig6, v113(v406, v5, v2)
;;                                     jump block16(v114)
;;
;;                                 block16(v22: i32):
;;                                     jump block26
;;
;;                                 block26:
;;                                     jump block28
;;
;;                                 block28:
;;                                     jump block30
;;
;;                                 block30:
;;                                     jump block32
;;
;;                                 block32:
;;                                     jump block37
;;
;;                                 block37:
;;                                     v77 = load.i64 notrap aligned readonly can_move region2 v5+72
;;                                     v80 = load.i64 notrap aligned readonly can_move region0 v77+8
;;                                     v81 = load.i64 notrap aligned readonly can_move region24 v80+104
;;                                     v407 = iconst.i64 8
;;                                     v83 = iadd v81, v407  ; v407 = 8
;;                                     v85 = load.i64 notrap aligned region25 v83
;;                                     jump block38
;;
;;                                 block38:
;;                                     jump block39
;;
;;                                 block39:
;;                                     v78 = uextend.i64 v22
;;                                     v86 = icmp ult v78, v85
;;                                     trapz v86, user12
;;                                     jump block34
;;
;;                                 block34:
;;                                     jump block40
;;
;;                                 block40:
;;                                     jump block41
;;
;;                                 block41:
;;                                     v94 = load.i64 notrap aligned region25 v81
;;                                     jump block42
;;
;;                                 block42:
;;                                     jump block43
;;
;;                                 block43:
;;                                     jump block31
;;
;;                                 block31:
;;                                     jump block44
;;
;;                                 block44:
;;                                     v379 = iconst.i64 4
;;                                     v380 = ishl.i64 v78, v379  ; v379 = 4
;;                                     v98 = iadd.i64 v94, v380
;;                                     v408 = iconst.i64 8
;;                                     v409 = iadd v98, v408  ; v408 = 8
;;                                     v102 = load.i64 notrap aligned region25 v409
;;                                     jump block29
;;
;;                                 block29:
;;                                     jump block45
;;
;;                                 block45:
;;                                     jump block27
;;
;;                                 block27:
;;                                     jump block22
;;
;;                                 block22:
;;                                     v106 = load.i64 notrap aligned region6 v37+136
;;                                     v107 = icmp eq v106, v36
;;                                     brif v107, block23, block24
;;
;;                                 block23:
;;                                     v108 = load.i64 notrap aligned region7 v36
;;                                     store notrap aligned region6 v108, v37+136
;;                                     v109 = load.i64 notrap aligned region15 v36+24
;;                                     store notrap aligned region14 v109, v37+152
;;                                     v110 = load.i32 notrap aligned region11 v36+16
;;                                     store notrap aligned region10 v110, v37+128
;;                                     v111 = load.i32 notrap aligned region13 v36+20
;;                                     store notrap aligned region12 v111, v37+132
;;                                     jump block25
;;
;;                                 block24 cold:
;;                                     v112 = load.i64 notrap aligned readonly can_move region3 v5+184
;;                                     v410 = load.i64 notrap aligned readonly can_move region2 v5+104
;;                                     call_indirect sig4, v112(v410, v5)
;;                                     jump block25
;;
;;                                 block25:
;;                                     jump block11
;;
;;                                 block11:
;;                                     jump block8
;;
;;                                 block8:
;;                                     jump block7
;;
;;                                 block7:
;;                                     jump block46
;;
;;                                 block46:
;; @0753                               v3 = iconst.i64 0
;;                                     v134 = stack_addr.i64 ss1
;;                                     v415 = load.i64 notrap aligned readonly can_move region2 v5+104
;;                                     v416 = iconst.i32 -1
;;                                     v417 = iadd.i32 v2, v416  ; v416 = -1
;;                                     v418 = uextend.i64 v417
;;                                     v419 = iconst.i64 24
;;                                     v420 = imul v418, v419  ; v419 = 24
;;                                     v425 = iconst.i32 2
;;                                     v429 = load.i64 notrap aligned readonly can_move region21 v37+144
;;                                     v430 = iconst.i64 3
;;                                     v433 = iconst.i64 1
;;                                     v263 = stack_addr.i64 ss2
;;                                     v375 = iconst.i8 1
;; @075f                               jump block2(v3)  ; v3 = 0
;;
;;                                 block2(v7: i64):
;; @0765                               v9 = icmp uge v7, v102
;; @0766                               brif v9, block4, block5
;;
;;                                 block4:
;; @076a                               v13 = load.i64 notrap aligned readonly can_move region3 v0+56
;; @076a                               v12 = load.i64 notrap aligned readonly can_move region2 v0+72
;; @076a                               call_indirect sig1, v13(v12, v0, v2)
;; @076c                               return
;;
;;                                 block5:
;; @0776                               jump block47
;;
;;                                 block47:
;;                                     jump block51
;;
;;                                 block51:
;;                                     v130 = load.i32 notrap aligned region5 v31
;;                                     trapz v130, user26
;;                                     jump block54
;;
;;                                 block54:
;;                                     v136 = load.i64 notrap aligned region6 v37+136
;;                                     store notrap aligned region7 v136, v134
;;                                     v411 = iconst.i32 0
;;                                     store notrap aligned region8 v411, v134+8  ; v411 = 0
;;                                     v412 = iconst.i32 1
;;                                     store notrap aligned region9 v412, v134+12  ; v412 = 1
;;                                     v137 = load.i32 notrap aligned region10 v37+128
;;                                     store notrap aligned region11 v137, v134+16
;;                                     store notrap aligned region10 v411, v37+128  ; v411 = 0
;;                                     v139 = load.i32 notrap aligned region12 v37+132
;;                                     store notrap aligned region13 v139, v134+20
;;                                     store notrap aligned region12 v411, v37+132  ; v411 = 0
;;                                     v141 = load.i64 notrap aligned region14 v37+152
;;                                     store notrap aligned region15 v141, v134+24
;;                                     store.i64 notrap aligned region6 v134, v37+136
;;                                     jump block58
;;
;;                                 block58:
;;                                     v413 = iconst.i64 8
;;                                     v414 = icmp.i64 ult v141, v413  ; v413 = 8
;;                                     brif v414, block59, block56
;;
;;                                 block59:
;;                                     v153 = load.i64 notrap aligned region16 v415+120
;;                                     v154 = icmp.i64 ult v418, v153
;;                                     brif v154, block60, block56
;;
;;                                 block60:
;;                                     v155 = load.i64 notrap aligned region17 v415+112
;;                                     v158 = iadd v155, v420
;;                                     v159 = load.i32 notrap aligned region18 v158
;;                                     v421 = iconst.i32 -1
;;                                     v422 = iadd v159, v421  ; v421 = -1
;;                                     v423 = iconst.i32 1
;;                                     v424 = icmp ule v422, v423  ; v423 = 1
;;                                     brif v424, block61, block56
;;
;;                                 block61:
;;                                     v164 = load.i32 notrap aligned region19 v158+4
;;                                     v426 = iconst.i32 2
;;                                     v427 = icmp eq v164, v426  ; v426 = 2
;;                                     brif v427, block62, block56
;;
;;                                 block62:
;;                                     v167 = load.i32 notrap aligned region20 v158+8
;;                                     v428 = iconst.i32 2
;;                                     v431 = iconst.i64 3
;;                                     v432 = ishl.i64 v141, v431  ; v431 = 3
;;                                     v171 = iadd.i64 v429, v432
;;                                     store notrap aligned region22 v428, v171  ; v428 = 2
;;                                     store.i32 notrap aligned region23 v2, v171+4
;;                                     v434 = iconst.i64 1
;;                                     v435 = iadd.i64 v141, v434  ; v434 = 1
;;                                     store notrap aligned region14 v435, v37+152
;;                                     jump block57(v167)
;;
;;                                 block56 cold:
;;                                     v436 = load.i64 notrap aligned readonly can_move region3 v5+152
;;                                     v245 = call_indirect sig20, v436(v415, v5, v2)
;;                                     jump block57(v245)
;;
;;                                 block57(v117: i32):
;;                                     jump block67
;;
;;                                 block67:
;;                                     jump block69
;;
;;                                 block69:
;;                                     jump block74
;;
;;                                 block74:
;;                                     jump block76
;;
;;                                 block76:
;;                                     jump block78
;;
;;                                 block78:
;;                                     jump block83
;;
;;                                 block83:
;;                                     v437 = iadd.i64 v81, v407  ; v407 = 8
;;                                     v183 = load.i64 notrap aligned region25 v437
;;                                     jump block84
;;
;;                                 block84:
;;                                     jump block85
;;
;;                                 block85:
;;                                     v176 = uextend.i64 v117
;;                                     v184 = icmp ult v176, v183
;;                                     trapz v184, user12
;;                                     jump block80
;;
;;                                 block80:
;;                                     jump block86
;;
;;                                 block86:
;;                                     jump block87
;;
;;                                 block87:
;;                                     v192 = load.i64 notrap aligned region25 v81
;;                                     jump block88
;;
;;                                 block88:
;;                                     jump block89
;;
;;                                 block89:
;;                                     jump block77
;;
;;                                 block77:
;;                                     jump block90
;;
;;                                 block90:
;;                                     v438 = iconst.i64 4
;;                                     v439 = ishl.i64 v176, v438  ; v438 = 4
;;                                     v196 = iadd.i64 v192, v439
;;                                     v440 = iconst.i64 8
;;                                     v441 = iadd v196, v440  ; v440 = 8
;;                                     v200 = load.i64 notrap aligned region25 v441
;;                                     jump block75
;;
;;                                 block75:
;;                                     jump block91
;;
;;                                 block91:
;;                                     v201 = icmp.i64 ult v7, v200
;;                                     trapz v201, user12
;;                                     jump block71
;;
;;                                 block71:
;;                                     jump block92
;;
;;                                 block92:
;;                                     jump block93
;;
;;                                 block93:
;;                                     jump block95
;;
;;                                 block95:
;;                                     jump block97
;;
;;                                 block97:
;;                                     jump block102
;;
;;                                 block102:
;;                                     jump block103
;;
;;                                 block103:
;;                                     jump block104
;;
;;                                 block104:
;;                                     jump block99
;;
;;                                 block99:
;;                                     jump block105
;;
;;                                 block105:
;;                                     jump block106
;;
;;                                 block106:
;;                                     jump block107
;;
;;                                 block107:
;;                                     jump block108
;;
;;                                 block108:
;;                                     jump block96
;;
;;                                 block96:
;;                                     jump block109
;;
;;                                 block109:
;;                                     v227 = load.i64 notrap aligned region25 v196
;;                                     jump block94
;;
;;                                 block94:
;;                                     jump block110
;;
;;                                 block110:
;;                                     v228 = iadd.i64 v227, v7
;;                                     v230 = load.i8 notrap aligned region25 v228
;;                                     jump block68
;;
;;                                 block68:
;;                                     jump block63
;;
;;                                 block63:
;;                                     v235 = load.i64 notrap aligned region6 v37+136
;;                                     v236 = icmp eq v235, v134
;;                                     brif v236, block64, block65
;;
;;                                 block64:
;;                                     v237 = load.i64 notrap aligned region7 v134
;;                                     store notrap aligned region6 v237, v37+136
;;                                     v238 = load.i64 notrap aligned region15 v134+24
;;                                     store notrap aligned region14 v238, v37+152
;;                                     v239 = load.i32 notrap aligned region11 v134+16
;;                                     store notrap aligned region10 v239, v37+128
;;                                     v240 = load.i32 notrap aligned region13 v134+20
;;                                     store notrap aligned region12 v240, v37+132
;;                                     jump block66
;;
;;                                 block65 cold:
;;                                     v442 = load.i64 notrap aligned readonly can_move region3 v5+184
;;                                     call_indirect sig18, v442(v415, v5)
;;                                     jump block66
;;
;;                                 block66:
;;                                     jump block52
;;
;;                                 block52:
;;                                     jump block49
;;
;;                                 block49:
;;                                     jump block48
;;
;;                                 block48:
;;                                     jump block111
;;
;;                                 block111:
;; @077b                               jump block112
;;
;;                                 block112:
;;                                     jump block116
;;
;;                                 block116:
;;                                     v259 = load.i32 notrap aligned region5 v31
;;                                     trapz v259, user26
;;                                     jump block119
;;
;;                                 block119:
;;                                     v265 = load.i64 notrap aligned region6 v37+136
;;                                     store notrap aligned region7 v265, v263
;;                                     v443 = iconst.i32 0
;;                                     store notrap aligned region8 v443, v263+8  ; v443 = 0
;;                                     v444 = iconst.i32 1
;;                                     store notrap aligned region9 v444, v263+12  ; v444 = 1
;;                                     v266 = load.i32 notrap aligned region10 v37+128
;;                                     store notrap aligned region11 v266, v263+16
;;                                     store notrap aligned region10 v443, v37+128  ; v443 = 0
;;                                     v268 = load.i32 notrap aligned region12 v37+132
;;                                     store notrap aligned region13 v268, v263+20
;;                                     store notrap aligned region12 v443, v37+132  ; v443 = 0
;;                                     v270 = load.i64 notrap aligned region14 v37+152
;;                                     store notrap aligned region15 v270, v263+24
;;                                     store.i64 notrap aligned region6 v263, v37+136
;;                                     jump block123
;;
;;                                 block123:
;;                                     v445 = iconst.i64 8
;;                                     v446 = icmp.i64 ult v270, v445  ; v445 = 8
;;                                     brif v446, block124, block121
;;
;;                                 block124:
;;                                     v282 = load.i64 notrap aligned region16 v415+120
;;                                     v283 = icmp.i64 ult v418, v282
;;                                     brif v283, block125, block121
;;
;;                                 block125:
;;                                     v284 = load.i64 notrap aligned region17 v415+112
;;                                     v287 = iadd v284, v420
;;                                     v288 = load.i32 notrap aligned region18 v287
;;                                     v447 = iconst.i32 -1
;;                                     v448 = iadd v288, v447  ; v447 = -1
;;                                     v449 = iconst.i32 1
;;                                     v450 = icmp ule v448, v449  ; v449 = 1
;;                                     brif v450, block126, block121
;;
;;                                 block126:
;;                                     v293 = load.i32 notrap aligned region19 v287+4
;;                                     v451 = iconst.i32 2
;;                                     v452 = icmp eq v293, v451  ; v451 = 2
;;                                     brif v452, block127, block121
;;
;;                                 block127:
;;                                     v296 = load.i32 notrap aligned region20 v287+8
;;                                     v453 = iconst.i32 2
;;                                     v454 = iconst.i64 3
;;                                     v455 = ishl.i64 v270, v454  ; v454 = 3
;;                                     v300 = iadd.i64 v429, v455
;;                                     store notrap aligned region22 v453, v300  ; v453 = 2
;;                                     store.i32 notrap aligned region23 v2, v300+4
;;                                     v456 = iconst.i64 1
;;                                     v457 = iadd.i64 v270, v456  ; v456 = 1
;;                                     store notrap aligned region14 v457, v37+152
;;                                     jump block122(v296)
;;
;;                                 block121 cold:
;;                                     v458 = load.i64 notrap aligned readonly can_move region3 v5+152
;;                                     v373 = call_indirect sig46, v458(v415, v5, v2)
;;                                     jump block122(v373)
;;
;;                                 block122(v248: i32):
;;                                     jump block132
;;
;;                                 block132:
;;                                     jump block134
;;
;;                                 block134:
;;                                     jump block139
;;
;;                                 block139:
;;                                     jump block141
;;
;;                                 block141:
;;                                     jump block143
;;
;;                                 block143:
;;                                     jump block148
;;
;;                                 block148:
;;                                     v459 = iadd.i64 v81, v407  ; v407 = 8
;;                                     v314 = load.i64 notrap aligned region25 v459
;;                                     jump block149
;;
;;                                 block149:
;;                                     jump block150
;;
;;                                 block150:
;;                                     v307 = uextend.i64 v248
;;                                     v315 = icmp ult v307, v314
;;                                     trapz v315, user12
;;                                     jump block145
;;
;;                                 block145:
;;                                     jump block151
;;
;;                                 block151:
;;                                     jump block152
;;
;;                                 block152:
;;                                     v323 = load.i64 notrap aligned region25 v81
;;                                     jump block153
;;
;;                                 block153:
;;                                     jump block154
;;
;;                                 block154:
;;                                     jump block142
;;
;;                                 block142:
;;                                     jump block155
;;
;;                                 block155:
;;                                     v460 = iconst.i64 4
;;                                     v461 = ishl.i64 v307, v460  ; v460 = 4
;;                                     v327 = iadd.i64 v323, v461
;;                                     v462 = iconst.i64 8
;;                                     v463 = iadd v327, v462  ; v462 = 8
;;                                     v331 = load.i64 notrap aligned region25 v463
;;                                     jump block140
;;
;;                                 block140:
;;                                     jump block156
;;
;;                                 block156:
;;                                     v332 = icmp.i64 ult v7, v331
;;                                     trapz v332, user12
;;                                     jump block136
;;
;;                                 block136:
;;                                     jump block157
;;
;;                                 block157:
;;                                     jump block158
;;
;;                                 block158:
;;                                     jump block160
;;
;;                                 block160:
;;                                     jump block162
;;
;;                                 block162:
;;                                     jump block167
;;
;;                                 block167:
;;                                     jump block168
;;
;;                                 block168:
;;                                     jump block169
;;
;;                                 block169:
;;                                     jump block164
;;
;;                                 block164:
;;                                     jump block170
;;
;;                                 block170:
;;                                     jump block171
;;
;;                                 block171:
;;                                     jump block172
;;
;;                                 block172:
;;                                     jump block173
;;
;;                                 block173:
;;                                     jump block161
;;
;;                                 block161:
;;                                     jump block174
;;
;;                                 block174:
;;                                     v358 = load.i64 notrap aligned region25 v327
;;                                     jump block159
;;
;;                                 block159:
;;                                     jump block175
;;
;;                                 block175:
;;                                     v464 = iconst.i8 1
;;                                     v465 = iadd.i8 v230, v464  ; v464 = 1
;;                                     v359 = iadd.i64 v358, v7
;;                                     store notrap aligned region25 v465, v359
;;                                     jump block133
;;
;;                                 block133:
;;                                     jump block128
;;
;;                                 block128:
;;                                     v365 = load.i64 notrap aligned region6 v37+136
;;                                     v366 = icmp eq v365, v263
;;                                     brif v366, block129, block130
;;
;;                                 block129:
;;                                     v367 = load.i64 notrap aligned region7 v263
;;                                     store notrap aligned region6 v367, v37+136
;;                                     v368 = load.i64 notrap aligned region15 v263+24
;;                                     store notrap aligned region14 v368, v37+152
;;                                     v369 = load.i32 notrap aligned region11 v263+16
;;                                     store notrap aligned region10 v369, v37+128
;;                                     v370 = load.i32 notrap aligned region13 v263+20
;;                                     store notrap aligned region12 v370, v37+132
;;                                     jump block131
;;
;;                                 block130 cold:
;;                                     v466 = load.i64 notrap aligned readonly can_move region3 v5+184
;;                                     call_indirect sig44, v466(v415, v5)
;;                                     jump block131
;;
;;                                 block131:
;;                                     jump block117
;;
;;                                 block117:
;;                                     jump block114
;;
;;                                 block114:
;;                                     jump block113
;;
;;                                 block113:
;;                                     jump block176
;;
;;                                 block176:
;;                                     v467 = iconst.i64 1
;; @0781                               v468 = iadd.i64 v7, v467  ; v467 = 1
;; @0784                               jump block2(v468)
;; }
