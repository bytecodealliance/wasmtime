;;! target = "x86_64"
;;! test = 'optimize'
;;! filter = "wasm[1]--function"

;; We cannot emit direct calls to imported functions when a module is exported,
;; since it might get linked with any number of different things at
;; runtime. Therefore the disassembly for function `g'` should have a
;; `call_indirect`, not a direct call to the adapter.

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

    (export "module" (core module $N))

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
  (export "module" (core module $b "module"))
)

;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @00ee                               v6 = load.i64 notrap aligned readonly can_move v0+48
;; @00ee                               v5 = load.i64 notrap aligned readonly can_move v0+64
;; @00eb                               v3 = iconst.i32 1234
;; @00ee                               v7 = call_indirect sig0, v6(v5, v0, v3)  ; v3 = 1234
;; @00f0                               jump block1
;;
;;                                 block1:
;; @00f0                               return v7
;; }
