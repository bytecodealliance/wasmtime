;;! target = "x86_64"
;;! flags = "-W gc,exceptions"
;;! test = "optimize"

(module
  (type $obj (struct))
  (tag $exn)
  (import "" "gc_then_throw" (func $gc_then_throw))

  (func (export "run") (result (ref null $obj))
    (local $local (ref null $obj))
    (local.set $local (struct.new $obj))
    block $outer_handler
      try_table (catch $exn $outer_handler)
        block $skip_throw
          block $inner_handler
            try_table (catch $exn $inner_handler)
              br $skip_throw
            end
          end
          throw $exn
        end
        call $gc_then_throw
      end
      ref.null $obj
      return
    end
    local.get $local)
)
;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 65 ""
;;     region3 = 177 ""
;;     region4 = 98 ""
;;     region5 = 130 ""
;;     region6 = 6 ""
;;     region7 = 196 ""
;;     region8 = 239 ""
;;     region9 = 134 ""
;;     region10 = 90 ""
;;     region11 = 21 ""
;;     region12 = 229 ""
;;     region13 = 96 ""
;;     region14 = 25 ""
;;     region15 = 68 ""
;;     region16 = 135 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx) -> i32 tail
;;     sig2 = (i64 vmctx, i32) -> i8 tail
;;     sig3 = (i64 vmctx, i64) tail
;;     fn0 = colocated u805306368:24 sig0
;;     fn1 = colocated u805306368:43 sig1
;;     fn2 = colocated u805306368:44 sig2
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0043                               v4 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @0043                               v5 = load.i32 notrap aligned region3 v4
;; @0043                               v6 = load.i32 notrap aligned region4 v4+4
;; @0043                               v12 = uextend.i64 v5
;;                                     v88 = iconst.i64 32
;; @0043                               v13 = iadd v12, v88  ; v88 = 32
;; @0043                               v14 = uextend.i64 v6
;; @0043                               v15 = icmp ule v13, v14
;; @0043                               brif v15, block2, block3
;;
;;                                 block2:
;;                                     v104 = iconst.i32 32
;;                                     v102 = iadd.i32 v5, v104  ; v104 = 32
;; @0043                               store notrap aligned region3 v102, v4
;;                                     v105 = iconst.i32 -1342177278
;;                                     v106 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v107 = load.i64 notrap aligned readonly can_move region7 v106+32
;; @0043                               v29 = iadd v107, v12
;; @0043                               store user2 region8 v105, v29  ; v105 = -1342177278
;;                                     v108 = load.i64 notrap aligned readonly can_move region5 v0+40
;;                                     v109 = load.i32 notrap aligned readonly can_move region6 v108
;; @0043                               store user2 region9 v109, v29+4
;;                                     v110 = iconst.i64 32
;; @0043                               istore32 user2 region10 v110, v29+8  ; v110 = 32
;; @0043                               jump block4(v5, v29)
;;
;;                                 block3 cold:
;; @0043                               v16 = iconst.i32 -1342177278
;; @0043                               v17 = load.i64 notrap aligned readonly can_move region5 v0+40
;; @0043                               v18 = load.i32 notrap aligned readonly can_move region6 v17
;; @0043                               v3 = iconst.i32 32
;; @0043                               v19 = iconst.i32 16
;; @0043                               v20 = call fn0(v0, v16, v18, v3, v19)  ; v16 = -1342177278, v3 = 32, v19 = 16
;; @0043                               v21 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0043                               v22 = load.i64 notrap aligned readonly can_move region7 v21+32
;; @0043                               v23 = uextend.i64 v20
;; @0043                               v24 = iadd v22, v23
;; @0043                               jump block4(v20, v24)
;;
;;                                 block4(v33: i32, v34: i64):
;;                                     v87 = stack_addr.i64 ss0
;;                                     store notrap aligned region16 v33, v87
;; @004a                               jump block6
;;
;;                                 block8(v35: i64):
;; @004a                               jump block5
;;
;;                                 block6:
;; @0054                               jump block11
;;
;;                                 block11:
;; @005a                               jump block9
;;
;;                                 block9:
;; @0061                               v78 = load.i64 notrap aligned readonly can_move region15 v0+56
;; @0061                               v77 = load.i64 notrap aligned readonly can_move region14 v0+72
;; @0061                               try_call_indirect v78(v77, v0), sig3, block18, [ context v0, tag0: block19(exn0) ], stack_map=[i32 @ ss0+0]
;;
;;                                 block19(v84: i64):
;;                                     v86 = load.i32 notrap aligned region16 v87
;;                                     jump block8(v84)
;;
;;                                 block18:
;; @0063                               jump block7
;;
;;                                 block7:
;; @0040                               v2 = iconst.i32 0
;; @0066                               return v2  ; v2 = 0
;;
;;                                 block5:
;; @006a                               jump block1
;;
;;                                 block1:
;; @006a                               return v86
;; }
