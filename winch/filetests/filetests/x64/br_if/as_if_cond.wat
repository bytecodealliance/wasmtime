;;! target = "x86_64"
(module
  (func (export "as-if-cond") (param i32) (result i32)
    (block (result i32)
      (if (result i32)
        (br_if 0 (i32.const 1) (local.get 0))
        (then (i32.const 2))
        (else (i32.const 3))
      )
    )
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c318000000       	add	r11, 0x18
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8740000000         	ja	0x5e
;;   1e:	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;      	 4889742408           	mov	qword ptr [rsp + 8], rsi
;;      	 89542404             	mov	dword ptr [rsp + 4], edx
;;      	 8b4c2404             	mov	ecx, dword ptr [rsp + 4]
;;      	 b801000000           	mov	eax, 1
;;      	 85c9                 	test	ecx, ecx
;;      	 0f8517000000         	jne	0x58
;;   41:	 85c0                 	test	eax, eax
;;      	 0f840a000000         	je	0x53
;;   49:	 b802000000           	mov	eax, 2
;;      	 e905000000           	jmp	0x58
;;   53:	 b803000000           	mov	eax, 3
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   5e:	 0f0b                 	ud2	
