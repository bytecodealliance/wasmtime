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
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c328000000       	add	r11, 0x28
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8781000000         	ja	0x9f
;;   1e:	 4883ec28             	sub	rsp, 0x28
;;      	 48897c2420           	mov	qword ptr [rsp + 0x20], rdi
;;      	 4889742418           	mov	qword ptr [rsp + 0x18], rsi
;;      	 4889542410           	mov	qword ptr [rsp + 0x10], rdx
;;      	 4531db               	xor	r11d, r11d
;;      	 4c895c2408           	mov	qword ptr [rsp + 8], r11
;;      	 4c891c24             	mov	qword ptr [rsp], r11
;;      	 48c7c001000000       	mov	rax, 1
;;      	 4889442408           	mov	qword ptr [rsp + 8], rax
;;      	 48c7c002000000       	mov	rax, 2
;;      	 48890424             	mov	qword ptr [rsp], rax
;;      	 488b442410           	mov	rax, qword ptr [rsp + 0x10]
;;      	 488b0c24             	mov	rcx, qword ptr [rsp]
;;      	 4839c1               	cmp	rcx, rax
;;      	 b900000000           	mov	ecx, 0
;;      	 400f97c1             	seta	cl
;;      	 85c9                 	test	ecx, ecx
;;      	 0f8523000000         	jne	0x94
;;   71:	 488b0424             	mov	rax, qword ptr [rsp]
;;      	 488b4c2408           	mov	rcx, qword ptr [rsp + 8]
;;      	 480fafc8             	imul	rcx, rax
;;      	 48894c2408           	mov	qword ptr [rsp + 8], rcx
;;      	 488b0424             	mov	rax, qword ptr [rsp]
;;      	 4883c001             	add	rax, 1
;;      	 48890424             	mov	qword ptr [rsp], rax
;;      	 e9c0ffffff           	jmp	0x54
;;   94:	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;      	 4883c428             	add	rsp, 0x28
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   9f:	 0f0b                 	ud2	
