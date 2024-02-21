;;! target = "x86_64"
(module
  (func (export "while-") (param i64) (result i64)
    (local i64)
    (local.set 1 (i64.const 1))
    (block
      (loop
        (br_if 1 (i64.eqz (local.get 0)))
        (local.set 1 (i64.mul (local.get 0) (local.get 1)))
        (local.set 0 (i64.sub (local.get 0) (i64.const 1)))
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
;;      	 4981c320000000       	add	r11, 0x20
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f876e000000         	ja	0x8c
;;   1e:	 4883ec20             	sub	rsp, 0x20
;;      	 48897c2418           	mov	qword ptr [rsp + 0x18], rdi
;;      	 4889742410           	mov	qword ptr [rsp + 0x10], rsi
;;      	 4889542408           	mov	qword ptr [rsp + 8], rdx
;;      	 48c7042400000000     	mov	qword ptr [rsp], 0
;;      	 48c7c001000000       	mov	rax, 1
;;      	 48890424             	mov	qword ptr [rsp], rax
;;      	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;      	 4883f800             	cmp	rax, 0
;;      	 b800000000           	mov	eax, 0
;;      	 400f94c0             	sete	al
;;      	 85c0                 	test	eax, eax
;;      	 0f8524000000         	jne	0x82
;;   5e:	 488b0424             	mov	rax, qword ptr [rsp]
;;      	 488b4c2408           	mov	rcx, qword ptr [rsp + 8]
;;      	 480fafc8             	imul	rcx, rax
;;      	 48890c24             	mov	qword ptr [rsp], rcx
;;      	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;      	 4883e801             	sub	rax, 1
;;      	 4889442408           	mov	qword ptr [rsp + 8], rax
;;      	 e9c2ffffff           	jmp	0x44
;;   82:	 488b0424             	mov	rax, qword ptr [rsp]
;;      	 4883c420             	add	rsp, 0x20
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   8c:	 0f0b                 	ud2	
