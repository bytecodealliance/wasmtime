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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108912 "VMStoreContext+0x30"
;;     region2 = 67108920 "VMStoreContext+0x38"
;;     region3 = 738197536 "VMComponentContext+0x20"
;;     region4 = 738197512 "VMComponentContext+0x8"
;;     region5 = 1879048208 "ComponentBuiltinFunctionsArray+0x10"
;;     region6 = 16 "VMContext+0x10"
;;     region7 = 1811939656 "BuiltinFunctionsArray+0x148"
;;     sig0 = (i64 sext, i32 sext, i32 sext, i32 sext) -> i64 sext system_v
;;     sig1 = (i64 sext vmctx) system_v
;;
;; block0(v0: i64, v1: i64, v2: i32):
;;     v4 = get_frame_pointer.i64 
;;     v3 = load.i64 notrap aligned readonly can_move region0 v1+8
;;     store notrap aligned region1 v4, v3+48
;;     v5 = get_return_address.i64 
;;     store notrap aligned region2 v5, v3+56
;;     v6 = load.i32 notrap aligned region3 v0+32
;;     trapz v6, user26
;;     v9 = load.i64 notrap aligned readonly region4 v0+8
;;     v10 = load.i64 notrap aligned readonly can_move region5 v9+16
;;     v7 = iconst.i32 0
;;     v11 = call_indirect sig0, v10(v0, v7, v7, v2)  ; v7 = 0, v7 = 0
;;     v12 = iconst.i64 -1
;;     v13 = icmp ne v11, v12  ; v12 = -1
;;     brif v13, block2, block1
;;
;; block1 cold:
;;     v14 = load.i64 notrap aligned readonly can_move region6 v1+16
;;     v15 = load.i64 notrap aligned readonly can_move region7 v14+328
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
