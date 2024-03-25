;;! target = "x86_64"
;;! test = "compile"
;;! flags = "-Ccranelift-has-bmi1 -Ccranelift-has-avx"

(module
  (func (export "blsi32") (param i32) (result i32)
    (i32.and
      (local.get 0)
      (i32.sub (i32.const 0) (local.get 0))))

  (func (export "blsi64") (param i64) (result i64)
    (i64.and
      (local.get 0)
      (i64.sub (i64.const 0) (local.get 0))))

  (func (export "blsr32") (param i32) (result i32)
    (i32.and
      (local.get 0)
      (i32.add (local.get 0) (i32.const -1))))

  (func (export "blsr64") (param i64) (result i64)
    (i64.and
      (local.get 0)
      (i64.sub (local.get 0) (i64.const 1))))

  (func (export "blsmsk32") (param i32) (result i32)
    (i32.xor
      (local.get 0)
      (i32.sub (local.get 0) (i32.const 1))))

  (func (export "blsmsk64") (param i64) (result i64)
    (i64.xor
      (local.get 0)
      (i64.add (local.get 0) (i64.const -1))))

  (func (export "tzcnt32") (param i32) (result i32)
    (i32.ctz (local.get 0)))

  (func (export "tzcnt64") (param i64) (result i64)
    (i64.ctz (local.get 0)))

  (func (export "andn32") (param i32 i32) (result i32)
    (i32.and (local.get 0) (i32.xor (local.get 1) (i32.const -1))))

  (func (export "andn64") (param i64 i64) (result i64)
    (i64.and (local.get 0) (i64.xor (local.get 1) (i64.const -1))))
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: blsil   %edx, %eax
;;    9: movq    %rbp, %rsp
;;    c: popq    %rbp
;;    d: retq
;;
;; wasm[0]::function[1]:
;;   10: pushq   %rbp
;;   11: movq    %rsp, %rbp
;;   14: blsiq   %rdx, %rax
;;   19: movq    %rbp, %rsp
;;   1c: popq    %rbp
;;   1d: retq
;;
;; wasm[0]::function[2]:
;;   20: pushq   %rbp
;;   21: movq    %rsp, %rbp
;;   24: blsrl   %edx, %eax
;;   29: movq    %rbp, %rsp
;;   2c: popq    %rbp
;;   2d: retq
;;
;; wasm[0]::function[3]:
;;   30: pushq   %rbp
;;   31: movq    %rsp, %rbp
;;   34: blsrq   %rdx, %rax
;;   39: movq    %rbp, %rsp
;;   3c: popq    %rbp
;;   3d: retq
;;
;; wasm[0]::function[4]:
;;   40: pushq   %rbp
;;   41: movq    %rsp, %rbp
;;   44: blsmskl %edx, %eax
;;   49: movq    %rbp, %rsp
;;   4c: popq    %rbp
;;   4d: retq
;;
;; wasm[0]::function[5]:
;;   50: pushq   %rbp
;;   51: movq    %rsp, %rbp
;;   54: blsmskq %rdx, %rax
;;   59: movq    %rbp, %rsp
;;   5c: popq    %rbp
;;   5d: retq
;;
;; wasm[0]::function[6]:
;;   60: pushq   %rbp
;;   61: movq    %rsp, %rbp
;;   64: tzcntl  %edx, %eax
;;   68: movq    %rbp, %rsp
;;   6b: popq    %rbp
;;   6c: retq
;;
;; wasm[0]::function[7]:
;;   70: pushq   %rbp
;;   71: movq    %rsp, %rbp
;;   74: tzcntq  %rdx, %rax
;;   79: movq    %rbp, %rsp
;;   7c: popq    %rbp
;;   7d: retq
;;
;; wasm[0]::function[8]:
;;   80: pushq   %rbp
;;   81: movq    %rsp, %rbp
;;   84: andnl   %edx, %ecx, %eax
;;   89: movq    %rbp, %rsp
;;   8c: popq    %rbp
;;   8d: retq
;;
;; wasm[0]::function[9]:
;;   90: pushq   %rbp
;;   91: movq    %rsp, %rbp
;;   94: andnq   %rdx, %rcx, %rax
;;   99: movq    %rbp, %rsp
;;   9c: popq    %rbp
;;   9d: retq
