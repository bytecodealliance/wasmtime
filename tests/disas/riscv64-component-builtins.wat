;;! target = "riscv64"
;;! test = 'optimize'
;;! filter = 'component-resource-drop[0]_wasm_call'

(component
  (type $a (resource (rep i32)))
  (core func $f (canon resource.drop $a))

  (core module $m (import "" "" (func (param i32))))
  (core instance (instantiate $m (with "" (instance (export "" (func $f))))))
)

;; function u0:0(i64 vmctx, i64, i32) tail {
;;     sig0 = (i64 sext, i32 sext, i32 sext) -> i64 sext system_v
;;     sig1 = (i64 sext vmctx) system_v
;;
;; block0(v0: i64, v1: i64, v2: i32):
;;     v3 = load.i32 notrap aligned little v0
;;     v16 = iconst.i32 0x706d_6f63
;;     v4 = icmp eq v3, v16  ; v16 = 0x706d_6f63
;;     trapz v4, user1
;;     v5 = load.i64 notrap aligned v0+16
;;     v6 = get_frame_pointer.i64 
;;     store notrap aligned v6, v5+40
;;     v7 = get_return_address.i64 
;;     store notrap aligned v7, v5+48
;;     v9 = load.i64 notrap aligned readonly v0+8
;;     v10 = load.i64 notrap aligned readonly v9+16
;;     v8 = iconst.i32 0
;;     v11 = call_indirect sig0, v10(v0, v8, v2)  ; v8 = 0
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
