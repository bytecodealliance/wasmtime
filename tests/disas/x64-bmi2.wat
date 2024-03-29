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
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       andl    $0x1f, %ecx
;;       bzhil   %ecx, %edx, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       andq    $0x3f, %rcx
;;       bzhiq   %rcx, %rdx, %rax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[2]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       rorxl   $8, %edx, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[3]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       rorxq   $0x37, %rdx, %rax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[4]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       shlxl   %ecx, %edx, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[5]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       shlxq   %rcx, %rdx, %rax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[6]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       shrxl   %ecx, %edx, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[7]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       shrxq   %rcx, %rdx, %rax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[8]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       sarxl   %ecx, %edx, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[9]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       sarxq   %rcx, %rdx, %rax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
