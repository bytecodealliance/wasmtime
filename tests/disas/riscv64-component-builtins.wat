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
;;     v17 = iconst.i32 0x706d_6f63
;;     v4 = icmp eq v3, v17  ; v17 = 0x706d_6f63
;;     trapz v4, user1
;;     v5 = load.i64 notrap aligned v0+16
;;     v6 = get_frame_pointer.i64 
;;     v7 = load.i64 notrap aligned v6
;;     store notrap aligned v7, v5+40
;;     v8 = get_return_address.i64 
;;     store notrap aligned v8, v5+48
;;     v10 = load.i64 notrap aligned readonly v0+8
;;     v11 = load.i64 notrap aligned readonly v10+16
;;     v9 = iconst.i32 0
;;     v12 = call_indirect sig0, v11(v0, v9, v2)  ; v9 = 0
;;     v13 = iconst.i64 -1
;;     v14 = icmp ne v12, v13  ; v13 = -1
;;     brif v14, block2, block1
;;
;; block1 cold:
;;     v15 = load.i64 notrap aligned readonly v1+16
;;     v16 = load.i64 notrap aligned readonly v15+408
;;     call_indirect sig1, v16(v1)
;;     trap user1
;;
;; block2:
;;     brif.i64 v12, block3, block4
;;
;; block3:
;;     jump block4
;;
;; block4:
;;     return
;; }
