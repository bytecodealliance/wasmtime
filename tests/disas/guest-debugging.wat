;;! target = "x86_64"
;;! test = "optimize"
;;! flags = ["-Dguest-debug=yes"]

(module
  (func (param i32) (result i32)
    (local $tmp0 i32)
    (local $tmp1 i32)

    i32.const 42
    local.set $tmp0

    local.get $tmp0
    local.get 0
    i32.add
    local.set $tmp1

    local.get $tmp1
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 28, key = 0
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 1543503872 "Stack(ss0)"
;;     sig0 = (i64 vmctx, i8) tail
;;     sig1 = (i64 vmctx) tail
;;     sig2 = (i64) preserve_all
;;     fn0 = colocated u805306368:40 sig0
;;     fn1 = colocated u805306368:41 sig1
;;     fn2 = colocated patchable u1073741824:46 sig2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0018                               v3 = stack_addr.i64 ss0+8
;; @0018                               store notrap region2 v2, v3
;; @0019                               v4 = iconst.i32 0
;; @0019                               v5 = stack_addr.i64 ss0+12
;; @0019                               store notrap region2 v4, v5  ; v4 = 0
;; @0019                               v6 = stack_addr.i64 ss0+16
;; @0019                               store notrap region2 v4, v6  ; v4 = 0
;; @0019                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0019                               v8 = load.i64 notrap aligned region1 v7+24
;; @0019                               v9 = get_stack_pointer.i64 
;; @0019                               v10 = icmp ult v9, v8
;; @0019                               brif v10, block2, block3
;;
;;                                 block2 cold:
;; @0019                               v11 = iconst.i8 0
;; @0019                               call fn0(v0, v11)  ; v11 = 0
;; <ss0, 25, 4294967295> @0019         call fn1(v0)
;; @0019                               trap user1
;;
;;                                 block3:
;; @0019                               v12 = stack_addr.i64 ss0
;; @0019                               store.i64 notrap region2 v0, v12
;; <ss0, 27, 4294967295> @001b         call fn2(v0)
;; @001b                               v13 = iconst.i32 42
;; @001b                               v14 = stack_addr.i64 ss0+20
;; @001b                               store notrap region2 v13, v14  ; v13 = 42
;; <ss0, 29, 0> @001d                  call fn2(v0)
;; @001d                               store notrap region2 v13, v5  ; v13 = 42
;; <ss0, 31, 4294967295> @001f         call fn2(v0)
;; @001f                               store notrap region2 v13, v14  ; v13 = 42
;; <ss0, 33, 0> @0021                  call fn2(v0)
;; @0021                               v17 = stack_addr.i64 ss0+24
;; @0021                               store.i32 notrap region2 v2, v17
;; <ss0, 35, 1> @0023                  call fn2(v0)
;;                                     v23 = iadd.i32 v2, v13  ; v13 = 42
;; @0023                               store notrap region2 v23, v14
;; <ss0, 36, 0> @0024                  call fn2(v0)
;; @0024                               store notrap region2 v23, v6
;; <ss0, 38, 4294967295> @0026         call fn2(v0)
;; @0026                               store notrap region2 v23, v14
;; <ss0, 40, 0> @0028                  call fn2(v0)
;; @0028                               jump block1
;;
;;                                 block1:
;;                                     v25 = iadd.i32 v2, v13  ; v13 = 42
;; @0028                               store notrap region2 v25, v14
;; @0028                               return v25
;; }
