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
;;      	 4883ec18             	sub	rsp, 0x18
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8769000000         	ja	0x81
;;   18:	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;      	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 48c7c001000000       	mov	rax, 1
;;      	 4889442408           	mov	qword ptr [rsp + 8], rax
;;      	 488b442410           	mov	rax, qword ptr [rsp + 0x10]
;;      	 4883f800             	cmp	rax, 0
;;      	 b800000000           	mov	eax, 0
;;      	 400f94c0             	sete	al
;;      	 85c0                 	test	eax, eax
;;      	 0f8526000000         	jne	0x76
;;   50:	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;      	 488b4c2410           	mov	rcx, qword ptr [rsp + 0x10]
;;      	 480fafc8             	imul	rcx, rax
;;      	 48894c2408           	mov	qword ptr [rsp + 8], rcx
;;      	 488b442410           	mov	rax, qword ptr [rsp + 0x10]
;;      	 4883e801             	sub	rax, 1
;;      	 4889442410           	mov	qword ptr [rsp + 0x10], rax
;;      	 e9c0ffffff           	jmp	0x36
;;   76:	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   81:	 0f0b                 	ud2	
