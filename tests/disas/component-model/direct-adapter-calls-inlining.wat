;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[1]--function"
;;! flags = "-C inlining=y -Wconcurrency-support=n"

;; Same as `direct-adapter-calls.wat`, except we have enabled function inlining
;; so all the direct calls should get inlined.

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

    (core module $N
      (import "" "f'" (func $f' (param i32) (result i32)))
      (func (export "g'") (result i32)
        (call $f' (i32.const 1234))
      )
    )

    (core instance $n
      (instantiate $N
        (with "" (instance (export "f'" (func $f'))))
      )
    )

    (func (export "g") (result u32)
      (canon lift (core func $n "g'"))
    )
  )

  (instance $a (instantiate $A))
  (instance $b
    (instantiate $B
      (with "f" (func $a "f"))
    )
  )

  (export "g" (func $b "g"))
)

;; function u1:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 72 "VMContext+0x48"
;;     region3 = 136 "VMContext+0x88"
;;     region4 = 1610612736 "PublicGlobal"
;;     region5 = 104 "VMContext+0x68"
;;     region6 = 88 "VMContext+0x58"
;;     region7 = 112 "VMContext+0x70"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned region1 gv4+24
;;     gv6 = vmctx
;;     gv7 = load.i64 notrap aligned readonly can_move region0 gv6+8
;;     gv8 = load.i64 notrap aligned region1 gv7+24
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64, i32) tail
;;     sig2 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u2:0 sig0
;;     fn1 = colocated u0:0 sig2
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @00ee                               jump block2
;;
;;                                 block2:
;;                                     jump block6
;;
;;                                 block8(v5: i64):
;;                                     jump block5
;;
;;                                 block6:
;; @00ee                               v3 = load.i64 notrap aligned readonly can_move region2 v0+72
;;                                     v9 = load.i64 notrap aligned readonly can_move region3 v3+136
;;                                     v10 = load.i32 notrap aligned region4 v9
;;                                     v11 = iconst.i32 1
;;                                     v12 = band v10, v11  ; v11 = 1
;;                                     v8 = iconst.i32 0
;;                                     v14 = icmp eq v12, v8  ; v8 = 0
;;                                     brif v14, block9, block10
;;
;;                                 block9:
;;                                     v18 = load.i64 notrap aligned readonly can_move region6 v3+88
;;                                     v17 = load.i64 notrap aligned readonly can_move region5 v3+104
;;                                     v16 = iconst.i32 23
;;                                     try_call_indirect v18(v17, v3, v16), sig1, block11, [ context v3, default: block8(exn0) ]  ; v16 = 23
;;
;;                                 block11:
;;                                     trap user12
;;
;;                                 block10:
;;                                     v23 = load.i64 notrap aligned readonly can_move region7 v3+112
;;                                     v24 = load.i32 notrap aligned region4 v23
;;                                     v25 = iconst.i32 -2
;;                                     v26 = band v24, v25  ; v25 = -2
;;                                     store notrap aligned region4 v26, v23
;;                                     v56 = iconst.i32 1
;;                                     v57 = bor v24, v56  ; v56 = 1
;;                                     store notrap aligned region4 v57, v23
;;                                     jump block13
;;
;;                                 block13:
;;                                     jump block14
;;
;;                                 block14:
;;                                     jump block12
;;
;;                                 block12:
;;                                     v37 = load.i32 notrap aligned region4 v9
;;                                     v58 = iconst.i32 -2
;;                                     v59 = band v37, v58  ; v58 = -2
;;                                     store notrap aligned region4 v59, v9
;;                                     v60 = iconst.i32 1
;;                                     v61 = bor v37, v60  ; v60 = 1
;;                                     store notrap aligned region4 v61, v9
;;                                     jump block7
;;
;;                                 block7:
;;                                     jump block4
;;
;;                                 block5:
;;                                     v20 = iconst.i32 49
;;                                     call_indirect.i64 sig1, v18(v17, v3, v20)  ; v20 = 49
;;                                     trap user12
;;
;;                                 block4:
;;                                     jump block3
;;
;;                                 block3:
;;                                     jump block15
;;
;;                                 block15:
;; @00f0                               jump block1
;;
;;                                 block1:
;;                                     v48 = iconst.i32 1276
;; @00f0                               return v48  ; v48 = 1276
;; }
