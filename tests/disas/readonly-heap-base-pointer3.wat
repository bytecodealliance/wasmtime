;;! test = "optimize"
;;! target = "x86_64"
;;! flags = ["-Wmemory64", "-Omemory-may-move=n"]

(module
  (memory i64 1)
  (func $load (param i64) (result i32)
    (i32.load (local.get 0)))
)
;; function u0:0(i64 vmctx, i64, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+96
;;     gv5 = load.i64 notrap aligned readonly checked gv3+88
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;; @0020                               v4 = iconst.i64 0xffff_fffc
;; @0020                               v5 = icmp ugt v2, v4  ; v4 = 0xffff_fffc
;; @0020                               v8 = iconst.i64 0
;; @0020                               v6 = load.i64 notrap aligned readonly checked v0+88
;; @0020                               v7 = iadd v6, v2
;; @0020                               v9 = select_spectre_guard v5, v8, v7  ; v8 = 0
;; @0020                               v10 = load.i32 little heap v9
;; @0023                               jump block1
;;
;;                                 block1:
;; @0023                               return v10
;; }
