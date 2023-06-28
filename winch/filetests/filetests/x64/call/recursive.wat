;;! target = "x86_64"

(module
  (func $fibonacci8 (param $n i32) (result i32)
    (if (result i32) (i32.le_s (local.get $n) (i32.const 1))
      (then
        ;; If n <= 1, return n (base case)
        (local.get $n)
      )
      (else
        ;; Else, return fibonacci(n - 1) + fibonacci(n - 2)
        (i32.add
          (call $fibonacci8
            (i32.sub (local.get $n) (i32.const 1)) ;; Calculate n - 1
          )
          (call $fibonacci8
            (i32.sub (local.get $n) (i32.const 2)) ;; Calculate n - 2
          )
        )
      )
    )
  )
  (export "fib" (func $fibonacci8))
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   11:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   15:	 83f801               	cmp	eax, 1
;;   18:	 b800000000           	mov	eax, 0
;;   1d:	 400f9ec0             	setle	al
;;   21:	 85c0                 	test	eax, eax
;;   23:	 0f8409000000         	je	0x32
;;   29:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   2d:	 e934000000           	jmp	0x66
;;   32:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   36:	 83e801               	sub	eax, 1
;;   39:	 50                   	push	rax
;;   3a:	 4883ec08             	sub	rsp, 8
;;   3e:	 8b7c2408             	mov	edi, dword ptr [rsp + 8]
;;   42:	 e800000000           	call	0x47
;;   47:	 4883c410             	add	rsp, 0x10
;;   4b:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   4f:	 83e902               	sub	ecx, 2
;;   52:	 50                   	push	rax
;;   53:	 51                   	push	rcx
;;   54:	 8b3c24               	mov	edi, dword ptr [rsp]
;;   57:	 e800000000           	call	0x5c
;;   5c:	 4883c408             	add	rsp, 8
;;   60:	 59                   	pop	rcx
;;   61:	 01c1                 	add	ecx, eax
;;   63:	 4889c8               	mov	rax, rcx
;;   66:	 4883c410             	add	rsp, 0x10
;;   6a:	 5d                   	pop	rbp
;;   6b:	 c3                   	ret	
