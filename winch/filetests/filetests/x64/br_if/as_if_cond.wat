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
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f873a000000         	ja	0x55
;;   1b:	 4883ec10             	sub	rsp, 0x10
;;      	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;      	 b801000000           	mov	eax, 1
;;      	 85c9                 	test	ecx, ecx
;;      	 0f8517000000         	jne	0x4f
;;   38:	 85c0                 	test	eax, eax
;;      	 0f840a000000         	je	0x4a
;;   40:	 b802000000           	mov	eax, 2
;;      	 e905000000           	jmp	0x4f
;;   4a:	 b803000000           	mov	eax, 3
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   55:	 0f0b                 	ud2	
