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
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c893424             	mov	qword ptr [rsp], r14
;;   10:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   15:	 4883ec04             	sub	rsp, 4
;;   19:	 44891c24             	mov	dword ptr [rsp], r11d
;;   1d:	 4883ec0c             	sub	rsp, 0xc
;;   21:	 bf01000000           	mov	edi, 1
;;   26:	 e800000000           	call	0x2b
;;   2b:	 4883c40c             	add	rsp, 0xc
;;   2f:	 4883ec04             	sub	rsp, 4
;;   33:	 890424               	mov	dword ptr [rsp], eax
;;   36:	 4883ec08             	sub	rsp, 8
;;   3a:	 bf01000000           	mov	edi, 1
;;   3f:	 e800000000           	call	0x44
;;   44:	 4883c408             	add	rsp, 8
;;   48:	 4883ec04             	sub	rsp, 4
;;   4c:	 890424               	mov	dword ptr [rsp], eax
;;   4f:	 8b0c24               	mov	ecx, dword ptr [rsp]
;;   52:	 4883c404             	add	rsp, 4
;;   56:	 8b0424               	mov	eax, dword ptr [rsp]
;;   59:	 4883c404             	add	rsp, 4
;;   5d:	 85c9                 	test	ecx, ecx
;;   5f:	 0f8409000000         	je	0x6e
;;   65:	 4883c404             	add	rsp, 4
;;   69:	 e902000000           	jmp	0x70
;;   6e:	 0f0b                 	ud2	
;;   70:	 4883c410             	add	rsp, 0x10
;;   74:	 5d                   	pop	rbp
;;   75:	 c3                   	ret	
