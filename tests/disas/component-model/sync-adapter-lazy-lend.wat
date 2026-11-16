;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[2]--function"
;;! flags = "-Wconcurrency-support=y"

;; An adapter that passes a `borrow` to the instance owning the resource, with
;; both halves of the lazy-lend fast path inline and no libcall on either.

(component
  (component $A
    (type $t' (resource (rep i32)))
    (export $t "t" (type $t'))

    (core func $ctor (canon resource.new $t))

    (core module $M
      (func (export "get'") (param i32) (result i32)
        (local.get 0)
      )
    )
    (core instance $m (instantiate $M))

    (func (export "[constructor]t") (param "x" u32) (result (own $t))
      (canon lift (core func $ctor))
    )
    (func (export "get") (param "self" (borrow $t)) (result u32)
      (canon lift (core func $m "get'"))
    )
  )
  (instance $a (instantiate $A))

  (component $B
    (import "a" (instance $i
      (export "t" (type $t (sub resource)))
      (export "get" (func (param "self" (borrow $t)) (result u32)))
    ))
    (alias export $i "t" (type $t))
    (alias export $i "get" (func $get))

    (core func $get' (canon lower (func $get)))

    (core module $M
      (import "" "get" (func $get (param i32) (result i32)))
      (func (export "run") (param i32) (result i32)
        (call $get (local.get 0))
      )
    )
    (core instance $m (instantiate $M
      (with "" (instance (export "get" (func $get'))))))

    (func (export "run") (param "x" u32) (result u32)
      (canon lift (core func $m "run"))
    )
  )
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "run" (func $b "run"))
)
;; function u2:0(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 32, align = 8
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 57 ""
;;     region3 = 209 ""
;;     region4 = 42 ""
;;     region5 = 68 ""
;;     region6 = 237 ""
;;     region7 = 126 ""
;;     region8 = 221 ""
;;     region9 = 112 ""
;;     region10 = 71 ""
;;     region11 = 149 ""
;;     region12 = 93 ""
;;     region13 = 30 ""
;;     region14 = 184 ""
;;     region15 = 108 ""
;;     region16 = 8 ""
;;     region17 = 218 ""
;;     region18 = 252 ""
;;     region19 = 13 ""
;;     region20 = 34 ""
;;     region21 = 195 ""
;;     region22 = 158 ""
;;     region23 = 174 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) tail
;;     sig1 = (i64 vmctx, i64, i32, i32) tail
;;     sig2 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u0:0 sig2
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @00e7                               jump block4
;;
;;                                 block6(v4: i64):
;; @00e7                               jump block3
;;
;;                                 block4:
;; @00ee                               v6 = load.i64 notrap aligned readonly can_move region2 v0+264
;; @00ee                               v7 = load.i32 notrap aligned region3 v6
;; @00f2                               trapz v7, user26
;; @00f2                               jump block7
;;
;;                                 block7:
;; @00fc                               v13 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @00fc                               v14 = load.i64 notrap aligned region5 v13+136
;; @00fc                               v12 = stack_addr.i64 ss0
;; @00fc                               store notrap aligned region6 v14, v12
;; @00e1                               v3 = iconst.i32 0
;; @00fc                               store notrap aligned region7 v3, v12+8  ; v3 = 0
;; @00fa                               v10 = iconst.i32 1
;; @00fc                               store notrap aligned region8 v10, v12+12  ; v10 = 1
;; @00fc                               v15 = load.i32 notrap aligned region9 v13+128
;; @00fc                               store notrap aligned region10 v15, v12+16
;; @00fc                               store notrap aligned region9 v3, v13+128  ; v3 = 0
;; @00fc                               v17 = load.i32 notrap aligned region11 v13+132
;; @00fc                               store notrap aligned region12 v17, v12+20
;; @00fc                               store notrap aligned region11 v3, v13+132  ; v3 = 0
;; @00fc                               v19 = load.i64 notrap aligned region13 v13+152
;; @00fc                               store notrap aligned region14 v19, v12+24
;; @00fc                               store notrap aligned region5 v12, v13+136
;; @0100                               jump block11
;;
;;                                 block11:
;; @0100                               v27 = iconst.i64 8
;; @0100                               v28 = icmp.i64 ult v19, v27  ; v27 = 8
;; @0100                               brif v28, block12, block9
;;
;;                                 block12:
;; @00f4                               v8 = load.i64 notrap aligned readonly can_move region4 v0+104
;; @0100                               v32 = load.i64 notrap aligned region15 v8+120
;; @0100                               v29 = iconst.i32 -1
;; @0100                               v30 = iadd.i32 v2, v29  ; v29 = -1
;; @0100                               v31 = uextend.i64 v30
;; @0100                               v33 = icmp ult v31, v32
;; @0100                               brif v33, block13, block9
;;
;;                                 block13:
;; @0100                               v34 = load.i64 notrap aligned region16 v8+112
;; @0100                               v35 = iconst.i64 24
;; @0100                               v36 = imul.i64 v31, v35  ; v35 = 24
;; @0100                               v37 = iadd v34, v36
;; @0100                               v38 = load.i32 notrap aligned region17 v37
;; @0100                               v74 = iconst.i32 -1
;; @0100                               v75 = iadd v38, v74  ; v74 = -1
;; @00fa                               v76 = iconst.i32 1
;; @0100                               v77 = icmp ule v75, v76  ; v76 = 1
;; @0100                               brif v77, block14, block9
;;
;;                                 block14:
;; @0100                               v43 = load.i32 notrap aligned region18 v37+4
;; @0100                               v44 = iconst.i32 2
;; @0100                               v45 = icmp eq v43, v44  ; v44 = 2
;; @0100                               brif v45, block15, block9
;;
;;                                 block15:
;; @0100                               v46 = load.i32 notrap aligned region19 v37+8
;; @0100                               v78 = iconst.i32 2
;; @0100                               v47 = load.i64 notrap aligned readonly can_move region20 v13+144
;;                                     v71 = iconst.i64 3
;;                                     v72 = ishl.i64 v19, v71  ; v71 = 3
;; @0100                               v50 = iadd v47, v72
;; @0100                               store notrap aligned region21 v78, v50  ; v78 = 2
;; @0100                               store.i32 notrap aligned region22 v2, v50+4
;; @0100                               v52 = iconst.i64 1
;; @0100                               v53 = iadd.i64 v19, v52  ; v52 = 1
;; @0100                               store notrap aligned region13 v53, v13+152
;; @0100                               jump block10(v46)
;;
;;                                 block9 cold:
;; @0100                               v54 = load.i64 notrap aligned readonly can_move region23 v0+152
;; @00f4                               v79 = load.i64 notrap aligned readonly can_move region4 v0+104
;; @0100                               v55 = call_indirect sig2, v54(v79, v0, v2)
;; @0100                               jump block10(v55)
;;
;;                                 block10(v23: i32):
;; @0102                               v56 = load.i64 notrap aligned readonly can_move region4 v0+72
;; @0102                               try_call fn0(v56, v0, v23), sig2, block16(ret0), [ context v0, default: block6(exn0) ]
;;
;;                                 block16(v57: i32):
;; @0106                               v61 = load.i64 notrap aligned region5 v13+136
;; @0106                               v62 = icmp eq v61, v12
;; @0106                               brif v62, block17, block18
;;
;;                                 block17:
;; @0106                               v63 = load.i64 notrap aligned region6 v12
;; @0106                               store notrap aligned region5 v63, v13+136
;; @0106                               v64 = load.i64 notrap aligned region14 v12+24
;; @0106                               store notrap aligned region13 v64, v13+152
;; @0106                               v65 = load.i32 notrap aligned region10 v12+16
;; @0106                               store notrap aligned region9 v65, v13+128
;; @0106                               v66 = load.i32 notrap aligned region12 v12+20
;; @0106                               store notrap aligned region11 v66, v13+132
;; @0106                               jump block19
;;
;;                                 block18 cold:
;; @0106                               v67 = load.i64 notrap aligned readonly can_move region23 v0+184
;; @00f4                               v80 = load.i64 notrap aligned readonly can_move region4 v0+104
;; @0106                               call_indirect sig0, v67(v80, v0)
;; @0106                               jump block19
;;
;;                                 block19:
;; @010a                               jump block5
;;
;;                                 block5:
;; @010b                               jump block2
;;
;;                                 block3:
;; @010e                               trap user52
;;
;;                                 block2:
;; @0112                               jump block1
;;
;;                                 block1:
;; @0112                               return v57
;; }
