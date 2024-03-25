;;! target = "x86_64"
;;! test = "compile"
;;! flags = [ "-Oopt-level=2", "-Cpcc=y", "-Ccranelift-has-avx=false" ]

(module
  (memory 1 1)
  (func (export "load_f32") (param i32) (result f32)
    local.get 0
    f32.load)
  (func (export "load_f64") (param i32) (result f64)
    local.get 0
    f64.load)
  (func (export "store_f32") (param i32 f32)
    local.get 0
    local.get 1
    f32.store)
  (func (export "store_f64") (param i32 f64)
    local.get 0
    local.get 1
    f64.store))
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    0x50(%rdi), %r9
;;    8: movl    %edx, %r10d
;;    b: movss   (%r9, %r10), %xmm0
;;   11: movq    %rbp, %rsp
;;   14: popq    %rbp
;;   15: retq
;;
;; wasm[0]::function[1]:
;;   20: pushq   %rbp
;;   21: movq    %rsp, %rbp
;;   24: movq    0x50(%rdi), %r9
;;   28: movl    %edx, %r10d
;;   2b: movsd   (%r9, %r10), %xmm0
;;   31: movq    %rbp, %rsp
;;   34: popq    %rbp
;;   35: retq
;;
;; wasm[0]::function[2]:
;;   40: pushq   %rbp
;;   41: movq    %rsp, %rbp
;;   44: movq    0x50(%rdi), %r9
;;   48: movl    %edx, %r10d
;;   4b: movss   %xmm0, (%r9, %r10)
;;   51: movq    %rbp, %rsp
;;   54: popq    %rbp
;;   55: retq
;;
;; wasm[0]::function[3]:
;;   60: pushq   %rbp
;;   61: movq    %rsp, %rbp
;;   64: movq    0x50(%rdi), %r9
;;   68: movl    %edx, %r10d
;;   6b: movsd   %xmm0, (%r9, %r10)
;;   71: movq    %rbp, %rsp
;;   74: popq    %rbp
;;   75: retq
