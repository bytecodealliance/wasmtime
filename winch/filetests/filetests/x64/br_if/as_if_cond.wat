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
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8736000000         	ja	0x4e
;;   18:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;      	 b801000000           	mov	eax, 1
;;      	 85c9                 	test	ecx, ecx
;;      	 0f8517000000         	jne	0x48
;;   31:	 85c0                 	test	eax, eax
;;      	 0f840a000000         	je	0x43
;;   39:	 b802000000           	mov	eax, 2
;;      	 e905000000           	jmp	0x48
;;   43:	 b803000000           	mov	eax, 3
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   4e:	 0f0b                 	ud2	
