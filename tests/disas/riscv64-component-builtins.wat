;;! target = "riscv64"
;;! test = 'optimize'
;;! filter = 'component_trampoline_0_Wasm'

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
;;     v20 = iconst.i32 0x706d_6f63
;;     v4 = icmp eq v3, v20  ; v20 = 0x706d_6f63
;;     trapz v4, user1
;;     v5 = load.i64 notrap aligned v0+16
;;     v7 = load.i64 notrap aligned v5+16
;;     v6 = get_stack_pointer.i64 
;;     v8 = icmp ult v6, v7
;;     trapnz v8, stk_ovf
;;     v9 = get_frame_pointer.i64 
;;     v10 = load.i64 notrap aligned v9
;;     store notrap aligned v10, v5+40
;;     v11 = get_return_address.i64 
;;     store notrap aligned v11, v5+48
;;     v13 = load.i64 notrap aligned readonly v0+8
;;     v14 = load.i64 notrap aligned readonly v13+16
;;     v12 = iconst.i32 0
;;     v15 = call_indirect sig0, v14(v0, v12, v2)  ; v12 = 0
;;     v16 = iconst.i64 -1
;;     v17 = icmp ne v15, v16  ; v16 = -1
;;     brif v17, block2, block1
;;
;; block1 cold:
;;     v18 = load.i64 notrap aligned readonly v1+16
;;     v19 = load.i64 notrap aligned readonly v18+416
;;     call_indirect sig1, v19(v1)
;;     trap user1
;;
;; block2:
;;     brif.i64 v15, block3, block4
;;
;; block3:
;;     jump block4
;;
;; block4:
;;     return
;; }
