;;! target = "riscv64"
;;! test = 'optimize'
;;! filter = 'wasm-call-component-resource-drop[0]'

(component
  (type $a (resource (rep i32)))
  (core func $f (canon resource.drop $a))

  (core module $m (import "" "" (func (param i32))))
  (core instance (instantiate $m (with "" (instance (export "" (func $f))))))
)

;; function u0:0(i64 vmctx, i64, i32) tail {
;;     sig0 = (i64 sext, i32 sext, i32 sext, i32 sext) -> i64 sext system_v
;;     sig1 = (i64 sext vmctx) system_v
;;
;; block0(v0: i64, v1: i64, v2: i32):
;;     v3 = load.i64 notrap aligned v0+16
;;     v4 = get_frame_pointer.i64 
;;     v5 = load.i64 notrap aligned v4
;;     store notrap aligned v5, v3+40
;;     v6 = get_return_address.i64 
;;     store notrap aligned v6, v3+48
;;     v9 = load.i64 notrap aligned readonly v0+8
;;     v10 = load.i64 notrap aligned readonly v9+16
;;     v7 = iconst.i32 0
;;     v11 = call_indirect sig0, v10(v0, v7, v7, v2)  ; v7 = 0, v7 = 0
;;     v12 = iconst.i64 -1
;;     v13 = icmp ne v11, v12  ; v12 = -1
;;     brif v13, block2, block1
;;
;; block1 cold:
;;     v14 = load.i64 notrap aligned readonly v1+16
;;     v15 = load.i64 notrap aligned readonly v14+408
;;     call_indirect sig1, v15(v1)
;;     trap user1
;;
;; block2:
;;     brif.i64 v11, block3, block4
;;
;; block3:
;;     jump block4
;;
;; block4:
;;     return
;; }
