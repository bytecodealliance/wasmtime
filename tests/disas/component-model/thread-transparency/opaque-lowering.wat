;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[1]--function"
;;! flags = "-C inlining=y -Wconcurrency-support=y"

;; Only the *lift* side of an adapter is judged, since FACT emits
;; `exit-sync-call` before translating results. The caller below declares and
;; uses `canon context.{get,set}`, yet its adapter into the clean callee is
;; still transparent and drops the `{enter,exit}-sync-call` calls.

(component
  (component $A
    (core module $M
      (func (export "f'") (param i32) (result i32)
        (i32.add (local.get 0) (i32.const 42))
      )
    )
    (core instance $m (instantiate $M))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f'"))
    )
  )

  (component $B
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $N
      (import "" "f'" (func $f' (param i32) (result i32)))
      (import "" "cget" (func $cget (result i32)))
      (import "" "cset" (func $cset (param i32)))
      (func (export "g'") (result i32) (local $r i32)
        (call $cset (i32.const 0x1234))
        (local.set $r (call $f' (i32.const 1234)))
        (i32.add (local.get $r) (call $cget))
      )
    )
    (core instance $n
      (instantiate $N
        (with "" (instance
          (export "f'" (func $f'))
          (export "cget" (func $cget))
          (export "cset" (func $cset))
        ))
      )
    )
    (func (export "g") (result u32)
      (canon lift (core func $n "g'"))
    )
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))

  (export "g" (func $b "g"))
)
;; function u1:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 1207959576 "VMFunctionImport+0x18"
;;     region3 = 67108992 "VMStoreContext+0x80"
;;     region4 = 1476395008 "VMGlobalImport+0x0"
;;     region5 = 738197568 "VMComponentContext+0x40"
;;     region6 = 738197552 "VMComponentContext+0x30"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned region1 gv4+24
;;     gv6 = vmctx
;;     gv7 = load.i64 notrap aligned readonly can_move region0 gv6+8
;;     gv8 = load.i64 notrap aligned region1 gv7+24
;;     sig0 = (i64 vmctx, i64, i32) tail
;;     sig1 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig2 = (i64 vmctx, i64) -> i32 tail
;;     sig3 = (i64 vmctx, i64) tail
;;     sig4 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u2147483648:18 sig0
;;     fn1 = colocated u2:0 sig1
;;     fn2 = colocated u2147483648:17 sig2
;;     fn3 = colocated u0:0 sig4
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0107                               v3 = iconst.i32 4660
;; @010a                               v5 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @010a                               store notrap aligned region3 v3, v5+128  ; v3 = 4660
;; @010f                               jump block2
;;
;;                                 block2:
;;                                     jump block6
;;
;;                                 block6:
;; @010f                               v7 = load.i64 notrap aligned readonly can_move region2 v0+72
;;                                     v17 = load.i64 notrap aligned readonly can_move region4 v7+168
;;                                     v18 = load.i32 notrap aligned region5 v17
;;                                     trapz v18, user26
;;                                     jump block9
;;
;;                                 block9:
;;                                     v19 = load.i64 notrap aligned readonly can_move region4 v7+144
;;                                     v20 = load.i32 notrap aligned region6 v19
;;                                     jump block12
;;
;;                                 block12:
;;                                     jump block13
;;
;;                                 block13:
;;                                     jump block11
;;
;;                                 block11:
;;                                     jump block7
;;
;;                                 block7:
;;                                     jump block4
;;
;;                                 block4:
;;                                     jump block3
;;
;;                                 block3:
;;                                     jump block14
;;
;;                                 block14:
;; @0118                               jump block1
;;
;;                                 block1:
;;                                     v37 = iconst.i32 5936
;; @0118                               return v37  ; v37 = 5936
;; }
