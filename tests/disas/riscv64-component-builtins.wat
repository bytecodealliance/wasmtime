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
;;     region0 = 2 "vmctx"
;;     sig0 = (i64 sext, i32 sext, i32 sext, i32 sext) -> i64 sext system_v
;;     sig1 = (i64 sext vmctx) system_v
;;
;; block0(v0: i64, v1: i64, v2: i32):
;;     v4 = get_frame_pointer.i64 
;;     v3 = load.i64 notrap aligned readonly can_move region0 v1+8
;;     store notrap aligned v4, v3+48
;;     v5 = get_return_address.i64 
;;     store notrap aligned v5, v3+56
;;     v6 = load.i32 notrap aligned v0+32
;;     v7 = iconst.i32 1
;;     v8 = band v6, v7  ; v7 = 1
;;     trapz v8, user26
;;     v11 = load.i64 notrap aligned readonly v0+8
;;     v12 = load.i64 notrap aligned readonly v11+16
;;     v9 = iconst.i32 0
;;     v13 = call_indirect sig0, v12(v0, v9, v9, v2)  ; v9 = 0, v9 = 0
;;     v14 = iconst.i64 -1
;;     v15 = icmp ne v13, v14  ; v14 = -1
;;     brif v15, block2, block1
;;
;; block1 cold:
;;     v16 = load.i64 notrap aligned readonly v1+16
;;     v17 = load.i64 notrap aligned readonly v16+328
;;     call_indirect sig1, v17(v1)
;;     trap user1
;;
;; block2:
;;     brif.i64 v13, block3, block4
;;
;; block3:
;;     jump block4
;;
;; block4:
;;     return
;; }
