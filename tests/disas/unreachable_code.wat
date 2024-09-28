;;! target = "x86_64"

(module
  (type (;0;) (func (param i32 i64 f64) (result f64)))
  (type (;1;) (func))
  (type (;2;) (func (result f32)))
  (type (;3;) (func (result f64)))
  (type (;4;) (func (param f64 f64) (result f64)))
  (type (;5;) (func (result i32)))
  (func (result i32)
      block (result i32)
        unreachable
      end
      block
      end
      i32.clz
  )
  (func (result i32)
      loop (result i32)
        unreachable
      end
      block
      end
      i32.clz
  )
  (func (;0;) (type 5) (result i32)
    nop
    block (result i32)  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          nop
          block  ;; label = @4
            i32.const 1
            if  ;; label = @5
              nop
              block  ;; label = @6
                nop
                nop
                loop (result i32)  ;; label = @7
                  nop
                  block (result i32)  ;; label = @8
                    nop
                    nop
                    block (result i32)  ;; label = @9
                      nop
                      unreachable
                    end
                  end
                end
                block (result i32)  ;; label = @7
                  block  ;; label = @8
                    nop
                  end
                  i32.const 0
                end
                br_if 5 (;@1;)
                drop
              end
            else
              nop
            end
            nop
          end
        end
      end
      unreachable
    end)
  (func
    block (result i32)
      block (result i32)
        i32.const 1
        br 1
      end
    end
    drop
  )
  (table (;0;) 16 funcref)
  (elem (i32.const 0))
)

;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0043                               trap user11
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @004c                               jump block2
;;
;;                                 block2:
;; @004e                               trap user11
;; }
;;
;; function u0:2(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0061                               v4 = iconst.i32 1
;; @0063                               brif v4, block6, block13  ; v4 = 1
;;
;;                                 block6:
;; @006a                               jump block9
;;
;;                                 block9:
;; @0074                               trap user11
;;
;;                                 block13:
;; @0087                               jump block7
;;
;;                                 block7:
;; @0089                               jump block5
;;
;;                                 block5:
;; @008a                               jump block4
;;
;;                                 block4:
;; @008b                               jump block3
;;
;;                                 block3:
;; @008c                               trap user11
;; }
;;
;; function u0:3(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0095                               v4 = iconst.i32 1
;; @0097                               jump block2(v4)  ; v4 = 1
;;
;;                                 block2(v2: i32):
;; @009c                               jump block1
;;
;;                                 block1:
;; @009c                               return
;; }
