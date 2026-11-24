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
;;     region0 = 123 ""
;;     region1 = 25 ""
;;     region2 = 135 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+16
;;     sig0 = (i64 vmctx) -> i8 tail
;;     sig1 = (i64) preserve_all
;;     fn0 = colocated u805306368:41 sig0
;;     fn1 = colocated patchable u1073741824:46 sig1
;;     stack_limit = gv2, handler = fn0
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0018                               v3 = stack_addr.i64 ss0+8
;; @0018                               store notrap region2 v2, v3
;; @0019                               v4 = iconst.i32 0
;; @0019                               v5 = stack_addr.i64 ss0+12
;; @0019                               store notrap region2 v4, v5  ; v4 = 0
;; @0019                               v6 = stack_addr.i64 ss0+16
;; @0019                               store notrap region2 v4, v6  ; v4 = 0
;; @0019                               v7 = stack_addr.i64 ss0
;; @0019                               store notrap region2 v0, v7
;; <ss0, 27, 4294967295> @001b         call fn1(v0)
;; @001b                               v8 = iconst.i32 42
;; @001b                               v9 = stack_addr.i64 ss0+20
;; @001b                               store notrap region2 v8, v9  ; v8 = 42
;; <ss0, 29, 0> @001d                  call fn1(v0)
;; @001d                               store notrap region2 v8, v5  ; v8 = 42
;; <ss0, 31, 4294967295> @001f         call fn1(v0)
;; @001f                               store notrap region2 v8, v9  ; v8 = 42
;; <ss0, 33, 0> @0021                  call fn1(v0)
;; @0021                               v12 = stack_addr.i64 ss0+24
;; @0021                               store notrap region2 v2, v12
;; <ss0, 35, 1> @0023                  call fn1(v0)
;;                                     v18 = iadd v2, v8  ; v8 = 42
;; @0023                               store notrap region2 v18, v9
;; <ss0, 36, 0> @0024                  call fn1(v0)
;; @0024                               store notrap region2 v18, v6
;; <ss0, 38, 4294967295> @0026         call fn1(v0)
;; @0026                               store notrap region2 v18, v9
;; <ss0, 40, 0> @0028                  call fn1(v0)
;; @0028                               jump block1
;;
;;                                 block1:
;;                                     v20 = iadd.i32 v2, v8  ; v8 = 42
;; @0028                               store notrap region2 v20, v9
;; @0028                               return v20
;; }
