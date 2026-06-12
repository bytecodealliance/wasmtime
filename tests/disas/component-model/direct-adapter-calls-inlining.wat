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
;;     region1 = 72 "VMContext+0x48"
;;     region2 = 136 "VMContext+0x88"
;;     region3 = 112 "VMContext+0x70"
;;     region4 = 1610612736 "PublicGlobal"
;;     region5 = 104 "VMContext+0x68"
;;     region6 = 88 "VMContext+0x58"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = vmctx
;;     gv5 = load.i64 notrap aligned readonly region0 gv4+8
;;     gv6 = load.i64 notrap aligned gv5+24
;;     gv7 = vmctx
;;     gv8 = load.i64 notrap aligned readonly can_move region2 gv7+136
;;     gv9 = load.i64 notrap aligned readonly can_move region3 gv7+112
;;     gv10 = vmctx
;;     gv11 = load.i64 notrap aligned readonly region0 gv10+8
;;     gv12 = load.i64 notrap aligned gv11+24
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
;;                                 block8(v10: i64):
;;                                     jump block5
;;
;;                                 block6:
;; @00ee                               v5 = load.i64 notrap aligned readonly can_move region1 v0+72
;;                                     v15 = load.i64 notrap aligned readonly can_move region2 v5+136
;;                                     v16 = load.i32 notrap aligned region4 v15
;;                                     v17 = iconst.i32 1
;;                                     v18 = band v16, v17  ; v17 = 1
;;                                     v14 = iconst.i32 0
;;                                     v20 = icmp eq v18, v14  ; v14 = 0
;;                                     brif v20, block9, block10
;;
;;                                 block9:
;;                                     v24 = load.i64 notrap aligned readonly can_move region6 v5+88
;;                                     v23 = load.i64 notrap aligned readonly can_move region5 v5+104
;;                                     v22 = iconst.i32 23
;;                                     try_call_indirect v24(v23, v5, v22), sig1, block11, [ context v5, default: block8(exn0) ]  ; v22 = 23
;;
;;                                 block11:
;;                                     trap user12
;;
;;                                 block10:
;;                                     v29 = load.i64 notrap aligned readonly can_move region3 v5+112
;;                                     v30 = load.i32 notrap aligned region4 v29
;;                                     v31 = iconst.i32 -2
;;                                     v32 = band v30, v31  ; v31 = -2
;;                                     store notrap aligned region4 v32, v29
;;                                     v62 = iconst.i32 1
;;                                     v63 = bor v30, v62  ; v62 = 1
;;                                     store notrap aligned region4 v63, v29
;;                                     jump block13
;;
;;                                 block13:
;;                                     jump block14
;;
;;                                 block14:
;;                                     jump block12
;;
;;                                 block12:
;;                                     v43 = load.i32 notrap aligned region4 v15
;;                                     v64 = iconst.i32 -2
;;                                     v65 = band v43, v64  ; v64 = -2
;;                                     store notrap aligned region4 v65, v15
;;                                     v66 = iconst.i32 1
;;                                     v67 = bor v43, v66  ; v66 = 1
;;                                     store notrap aligned region4 v67, v15
;;                                     jump block7
;;
;;                                 block7:
;;                                     jump block4
;;
;;                                 block5:
;;                                     v26 = iconst.i32 49
;;                                     call_indirect.i64 sig1, v24(v23, v5, v26)  ; v26 = 49
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
;;                                     v54 = iconst.i32 1276
;; @00f0                               return v54  ; v54 = 1276
;; }
