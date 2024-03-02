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
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4c8b5f08             	mov	r11, qword ptr [rdi + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c350000000       	add	r11, 0x50
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f87d9000000         	ja	0xf4
;;   1b:	 4883ec30             	sub	rsp, 0x30
;;      	 48891c24             	mov	qword ptr [rsp], rbx
;;      	 4c89642408           	mov	qword ptr [rsp + 8], r12
;;      	 4c896c2410           	mov	qword ptr [rsp + 0x10], r13
;;      	 4c89742418           	mov	qword ptr [rsp + 0x18], r14
;;      	 4c897c2420           	mov	qword ptr [rsp + 0x20], r15
;;      	 4989fe               	mov	r14, rdi
;;      	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2440           	mov	qword ptr [rsp + 0x40], rdi
;;      	 4889742438           	mov	qword ptr [rsp + 0x38], rsi
;;      	 89542434             	mov	dword ptr [rsp + 0x34], edx
;;      	 8b442434             	mov	eax, dword ptr [rsp + 0x34]
;;      	 83f801               	cmp	eax, 1
;;      	 b800000000           	mov	eax, 0
;;      	 400f9ec0             	setle	al
;;      	 85c0                 	test	eax, eax
;;      	 0f8409000000         	je	0x6d
;;   64:	 8b442434             	mov	eax, dword ptr [rsp + 0x34]
;;      	 e965000000           	jmp	0xd2
;;   6d:	 8b442434             	mov	eax, dword ptr [rsp + 0x34]
;;      	 83e801               	sub	eax, 1
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 4883ec04             	sub	rsp, 4
;;      	 4c89f7               	mov	rdi, r14
;;      	 4c89f6               	mov	rsi, r14
;;      	 8b542404             	mov	edx, dword ptr [rsp + 4]
;;      	 e800000000           	call	0x8e
;;      	 4883c404             	add	rsp, 4
;;      	 4883c404             	add	rsp, 4
;;      	 4c8b742440           	mov	r14, qword ptr [rsp + 0x40]
;;      	 8b4c2434             	mov	ecx, dword ptr [rsp + 0x34]
;;      	 83e902               	sub	ecx, 2
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 4883ec04             	sub	rsp, 4
;;      	 890c24               	mov	dword ptr [rsp], ecx
;;      	 4c89f7               	mov	rdi, r14
;;      	 4c89f6               	mov	rsi, r14
;;      	 8b1424               	mov	edx, dword ptr [rsp]
;;      	 e800000000           	call	0xbe
;;      	 4883c404             	add	rsp, 4
;;      	 4c8b742444           	mov	r14, qword ptr [rsp + 0x44]
;;      	 8b0c24               	mov	ecx, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 01c1                 	add	ecx, eax
;;      	 89c8                 	mov	eax, ecx
;;      	 4883c418             	add	rsp, 0x18
;;      	 488b1c24             	mov	rbx, qword ptr [rsp]
;;      	 4c8b642408           	mov	r12, qword ptr [rsp + 8]
;;      	 4c8b6c2410           	mov	r13, qword ptr [rsp + 0x10]
;;      	 4c8b742418           	mov	r14, qword ptr [rsp + 0x18]
;;      	 4c8b7c2420           	mov	r15, qword ptr [rsp + 0x20]
;;      	 4883c430             	add	rsp, 0x30
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   f4:	 0f0b                 	ud2	
