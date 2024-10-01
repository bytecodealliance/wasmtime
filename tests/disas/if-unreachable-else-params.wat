;;! target = "x86_64"

(module
  (type (;0;) (func (param i32)))
  (func $main (type 0) (param i32)
    i32.const 35
    loop (param i32)  ;; label = @1
      local.get 0
      if (param i32)  ;; label = @2
        i64.load16_s align=1
        unreachable
        unreachable
        unreachable
        unreachable
        unreachable
        local.get 0
        unreachable
        unreachable
        i64.load8_u offset=11789
        unreachable
      else
        i32.popcnt
        local.set 0
        return
        unreachable
      end
      unreachable
      unreachable
      nop
      f32.lt
      i32.store8 offset=82
      unreachable
    end
    unreachable
    unreachable
    unreachable
    unreachable)
  (table (;0;) 63 255 funcref)
  (memory (;0;) 13 16)
  (export "t1" (table 0))
  (export "m1" (memory 0))
  (export "main" (func $main))
  (export "memory" (memory 0)))

;; function u0:0(i64 vmctx, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly checked gv3+112
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0043                               v3 = iconst.i32 35
;; @0045                               jump block2(v3)  ; v3 = 35
;;
;;                                 block2(v4: i32):
;; @0049                               brif.i32 v2, block4, block6(v4)
;;
;;                                 block4:
;; @004b                               v7 = uextend.i64 v4
;; @004b                               v8 = global_value.i64 gv4
;; @004b                               v9 = iadd v8, v7
;; @004b                               v10 = sload16.i64 little heap v9
;; @004e                               trap user11
;;
;;                                 block6(v6: i32):
;; @005d                               v11 = popcnt.i32 v4
;; @0060                               return
;; }
