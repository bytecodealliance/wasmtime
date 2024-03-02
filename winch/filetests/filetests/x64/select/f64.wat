;;! target = "x86_64"

(module
  (func (export "select-f64") (param f64 f64 i32) (result f64)
    (select (local.get 0) (local.get 1) (local.get 2))
  )
)
 
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4c8b5f08             	mov	r11, qword ptr [rdi + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c358000000       	add	r11, 0x58
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f877c000000         	ja	0x97
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
;;      	 f20f11442440         	movsd	qword ptr [rsp + 0x40], xmm0
;;      	 f20f114c2438         	movsd	qword ptr [rsp + 0x38], xmm1
;;      	 89542434             	mov	dword ptr [rsp + 0x34], edx
;;      	 8b442434             	mov	eax, dword ptr [rsp + 0x34]
;;      	 f20f10442438         	movsd	xmm0, qword ptr [rsp + 0x38]
;;      	 f20f104c2440         	movsd	xmm1, qword ptr [rsp + 0x40]
;;      	 83f800               	cmp	eax, 0
;;      	 0f8404000000         	je	0x75
;;   71:	 f20f10c1             	movsd	xmm0, xmm1
;;      	 4883c428             	add	rsp, 0x28
;;      	 488b1c24             	mov	rbx, qword ptr [rsp]
;;      	 4c8b642408           	mov	r12, qword ptr [rsp + 8]
;;      	 4c8b6c2410           	mov	r13, qword ptr [rsp + 0x10]
;;      	 4c8b742418           	mov	r14, qword ptr [rsp + 0x18]
;;      	 4c8b7c2420           	mov	r15, qword ptr [rsp + 0x20]
;;      	 4883c430             	add	rsp, 0x30
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   97:	 0f0b                 	ud2	
