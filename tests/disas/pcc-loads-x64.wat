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
;; function u0:0:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   movq    80(%rdi), %r9
;;   movl    %edx, %r10d
;;   movss   0(%r9,%r10,1), %xmm0
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:1:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   movq    80(%rdi), %r9
;;   movl    %edx, %r10d
;;   movsd   0(%r9,%r10,1), %xmm0
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:2:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   movq    80(%rdi), %r9
;;   movl    %edx, %r10d
;;   movss   %xmm0, 0(%r9,%r10,1)
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:3:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   movq    80(%rdi), %r9
;;   movl    %edx, %r10d
;;   movsd   %xmm0, 0(%r9,%r10,1)
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
