;;! target = "x86_64"
;;! test = "compile"
;;! flags = "-Ccranelift-has-bmi2 -Ccranelift-has-avx"

(module
  (func (export "bzhi32") (param i32 i32) (result i32)
    (i32.and
      (local.get 0)
      (i32.sub
        (i32.shl
          (i32.const 1)
          (local.get 1))
        (i32.const 1))))

  (func (export "bzhi64") (param i64 i64) (result i64)
    (i64.and
      (local.get 0)
      (i64.add
        (i64.shl
          (i64.const 1)
          (local.get 1))
        (i64.const -1))))

  (func (export "rorx32") (param i32) (result i32)
    (i32.rotr (local.get 0) (i32.const 8)))

  (func (export "rorx64") (param i64) (result i64)
    (i64.rotl (local.get 0) (i64.const 9)))

  (func (export "shlx32") (param i32 i32) (result i32)
    (i32.shl (local.get 0) (local.get 1)))
  (func (export "shlx64") (param i64 i64) (result i64)
    (i64.shl (local.get 0) (local.get 1)))

  (func (export "shrx32") (param i32 i32) (result i32)
    (i32.shr_u (local.get 0) (local.get 1)))
  (func (export "shrx64") (param i64 i64) (result i64)
    (i64.shr_u (local.get 0) (local.get 1)))

  (func (export "sarx32") (param i32 i32) (result i32)
    (i32.shr_s (local.get 0) (local.get 1)))
  (func (export "sarx64") (param i64 i64) (result i64)
    (i64.shr_s (local.get 0) (local.get 1)))
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: andl    $0x1f, %ecx
;;    7: bzhil   %ecx, %edx, %eax
;;    c: movq    %rbp, %rsp
;;    f: popq    %rbp
;;   10: retq
;;
;; wasm[0]::function[1]:
;;   20: pushq   %rbp
;;   21: movq    %rsp, %rbp
;;   24: andq    $0x3f, %rcx
;;   28: bzhiq   %rcx, %rdx, %rax
;;   2d: movq    %rbp, %rsp
;;   30: popq    %rbp
;;   31: retq
;;
;; wasm[0]::function[2]:
;;   40: pushq   %rbp
;;   41: movq    %rsp, %rbp
;;   44: rorxl   $8, %edx, %eax
;;   4a: movq    %rbp, %rsp
;;   4d: popq    %rbp
;;   4e: retq
;;
;; wasm[0]::function[3]:
;;   50: pushq   %rbp
;;   51: movq    %rsp, %rbp
;;   54: rorxq   $0x37, %rdx, %rax
;;   5a: movq    %rbp, %rsp
;;   5d: popq    %rbp
;;   5e: retq
;;
;; wasm[0]::function[4]:
;;   60: pushq   %rbp
;;   61: movq    %rsp, %rbp
;;   64: shlxl   %ecx, %edx, %eax
;;   69: movq    %rbp, %rsp
;;   6c: popq    %rbp
;;   6d: retq
;;
;; wasm[0]::function[5]:
;;   70: pushq   %rbp
;;   71: movq    %rsp, %rbp
;;   74: shlxq   %rcx, %rdx, %rax
;;   79: movq    %rbp, %rsp
;;   7c: popq    %rbp
;;   7d: retq
;;
;; wasm[0]::function[6]:
;;   80: pushq   %rbp
;;   81: movq    %rsp, %rbp
;;   84: shrxl   %ecx, %edx, %eax
;;   89: movq    %rbp, %rsp
;;   8c: popq    %rbp
;;   8d: retq
;;
;; wasm[0]::function[7]:
;;   90: pushq   %rbp
;;   91: movq    %rsp, %rbp
;;   94: shrxq   %rcx, %rdx, %rax
;;   99: movq    %rbp, %rsp
;;   9c: popq    %rbp
;;   9d: retq
;;
;; wasm[0]::function[8]:
;;   a0: pushq   %rbp
;;   a1: movq    %rsp, %rbp
;;   a4: sarxl   %ecx, %edx, %eax
;;   a9: movq    %rbp, %rsp
;;   ac: popq    %rbp
;;   ad: retq
;;
;; wasm[0]::function[9]:
;;   b0: pushq   %rbp
;;   b1: movq    %rsp, %rbp
;;   b4: sarxq   %rcx, %rdx, %rax
;;   b9: movq    %rbp, %rsp
;;   bc: popq    %rbp
;;   bd: retq
