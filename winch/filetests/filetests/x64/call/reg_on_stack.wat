;;! target = "x86_64"
(module
  (func (export "") (param i32) (result i32)
    local.get 0
    i32.const 1
    call 0
    i32.const 1
    call 0
    br_if 0 (;@0;)
    unreachable
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f876e000000         	ja	0x86
;;   18:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 4883ec0c             	sub	rsp, 0xc
;;      	 bf01000000           	mov	edi, 1
;;      	 e800000000           	call	0x3b
;;      	 4883c40c             	add	rsp, 0xc
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 4883ec08             	sub	rsp, 8
;;      	 bf01000000           	mov	edi, 1
;;      	 e800000000           	call	0x54
;;      	 4883c408             	add	rsp, 8
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 8b0c24               	mov	ecx, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 8b0424               	mov	eax, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 85c9                 	test	ecx, ecx
;;      	 0f8409000000         	je	0x7e
;;   75:	 4883c404             	add	rsp, 4
;;      	 e902000000           	jmp	0x80
;;   7e:	 0f0b                 	ud2	
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   86:	 0f0b                 	ud2	
