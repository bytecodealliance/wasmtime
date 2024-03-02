;;! target = "x86_64"
(module
  (func (export "for-") (param i64) (result i64)
    (local i64 i64)
    (local.set 1 (i64.const 1))
    (local.set 2 (i64.const 2))
    (block
      (loop
        (br_if 1 (i64.gt_u (local.get 2) (local.get 0)))
        (local.set 1 (i64.mul (local.get 1) (local.get 2)))
        (local.set 2 (i64.add (local.get 2) (i64.const 1)))
        (br 0)
      )
    )
    (local.get 1)
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4c8b5f08             	mov	r11, qword ptr [rdi + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c358000000       	add	r11, 0x58
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f87c2000000         	ja	0xdd
;;   1b:	 4883ec30             	sub	rsp, 0x30
;;      	 48891c24             	mov	qword ptr [rsp], rbx
;;      	 4c89642408           	mov	qword ptr [rsp + 8], r12
;;      	 4c896c2410           	mov	qword ptr [rsp + 0x10], r13
;;      	 4c89742418           	mov	qword ptr [rsp + 0x18], r14
;;      	 4c897c2420           	mov	qword ptr [rsp + 0x20], r15
;;      	 4989fe               	mov	r14, rdi
;;      	 4883ec28             	sub	rsp, 0x28
;;      	 48897c2450           	mov	qword ptr [rsp + 0x50], rdi
;;      	 4889742448           	mov	qword ptr [rsp + 0x48], rsi
;;      	 4889542440           	mov	qword ptr [rsp + 0x40], rdx
;;      	 4531db               	xor	r11d, r11d
;;      	 4c895c2438           	mov	qword ptr [rsp + 0x38], r11
;;      	 4c895c2430           	mov	qword ptr [rsp + 0x30], r11
;;      	 48c7c001000000       	mov	rax, 1
;;      	 4889442438           	mov	qword ptr [rsp + 0x38], rax
;;      	 48c7c002000000       	mov	rax, 2
;;      	 4889442430           	mov	qword ptr [rsp + 0x30], rax
;;      	 488b442440           	mov	rax, qword ptr [rsp + 0x40]
;;      	 488b4c2430           	mov	rcx, qword ptr [rsp + 0x30]
;;      	 4839c1               	cmp	rcx, rax
;;      	 b900000000           	mov	ecx, 0
;;      	 400f97c1             	seta	cl
;;      	 85c9                 	test	ecx, ecx
;;      	 0f8526000000         	jne	0xb6
;;   90:	 488b442430           	mov	rax, qword ptr [rsp + 0x30]
;;      	 488b4c2438           	mov	rcx, qword ptr [rsp + 0x38]
;;      	 480fafc8             	imul	rcx, rax
;;      	 48894c2438           	mov	qword ptr [rsp + 0x38], rcx
;;      	 488b442430           	mov	rax, qword ptr [rsp + 0x30]
;;      	 4883c001             	add	rax, 1
;;      	 4889442430           	mov	qword ptr [rsp + 0x30], rax
;;      	 e9bcffffff           	jmp	0x72
;;   b6:	 488b442438           	mov	rax, qword ptr [rsp + 0x38]
;;      	 4883c428             	add	rsp, 0x28
;;      	 488b1c24             	mov	rbx, qword ptr [rsp]
;;      	 4c8b642408           	mov	r12, qword ptr [rsp + 8]
;;      	 4c8b6c2410           	mov	r13, qword ptr [rsp + 0x10]
;;      	 4c8b742418           	mov	r14, qword ptr [rsp + 0x18]
;;      	 4c8b7c2420           	mov	r15, qword ptr [rsp + 0x20]
;;      	 4883c430             	add	rsp, 0x30
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   dd:	 0f0b                 	ud2	
