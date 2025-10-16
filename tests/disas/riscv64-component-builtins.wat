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
;;     store notrap aligned v4, v3+40
;;     v5 = get_return_address.i64 
;;     store notrap aligned v5, v3+48
;;     v8 = load.i64 notrap aligned readonly v0+8
;;     v9 = load.i64 notrap aligned readonly v8+16
;;     v6 = iconst.i32 0
;;     v10 = call_indirect sig0, v9(v0, v6, v6, v2)  ; v6 = 0, v6 = 0
;;     v11 = iconst.i64 -1
;;     v12 = icmp ne v10, v11  ; v11 = -1
;;     brif v12, block2, block1
;;
;; block1 cold:
;;     v13 = load.i64 notrap aligned readonly v1+16
;;     v14 = load.i64 notrap aligned readonly v13+408
;;     call_indirect sig1, v14(v1)
;;     trap user1
;;
;; block2:
;;     brif.i64 v10, block3, block4
;;
;; block3:
;;     jump block4
;;
;; block4:
;;     return
;; }
