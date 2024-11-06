;;! test = "optimize"
;;! target = "x86_64"
;;! flags = ["-Omemory-reservation=0x20000", "-Omemory-may-move=n"]

(module
  (memory 1)
  (func $load (param i32) (result i32)
    (i32.load (local.get 0)))
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0020                               v4 = uextend.i64 v2
;; @0020                               v5 = iconst.i64 0x0001_fffc
;; @0020                               v6 = icmp ugt v4, v5  ; v5 = 0x0001_fffc
;; @0020                               v9 = iconst.i64 0
;; @0020                               v7 = load.i64 notrap aligned readonly checked v0+96
;; @0020                               v8 = iadd v7, v4
;; @0020                               v10 = select_spectre_guard v6, v9, v8  ; v9 = 0
;; @0020                               v11 = load.i32 little heap v10
;; @0023                               jump block1
;;
;;                                 block1:
;; @0023                               return v11
;; }
