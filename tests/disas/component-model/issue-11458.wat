;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[1]--function"
;;! flags = "-C inlining=y"

(component
  (core module $m
    (func $f (export "f") (param i32) (result i32)
      (return_call $f (i32.add (local.get 0) (i32.const 1)))
    )
  )
  (core module $n
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32)
      (call $f (i32.const 0))
    )
  )
  (core instance $i (instantiate $m))
  (core instance $j (instantiate $n (with "" (instance $i))))
)

;; function u1:0(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 1207959576 "VMFunctionImport+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned region1 gv4+24
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u0:0 sig0
;;     fn1 = colocated u0:0 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @006f                               jump block2
;;
;;                                 block2:
;; @006f                               v4 = load.i64 notrap aligned readonly can_move region2 v0+72
;;                                     v7 = iconst.i32 1
;;                                     v9 = call fn1(v4, v4, v7)  ; v7 = 1
;;                                     jump block4
;;
;;                                 block4:
;; @0071                               jump block1
;;
;;                                 block1:
;; @0071                               return v9
;; }
