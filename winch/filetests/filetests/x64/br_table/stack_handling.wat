;;! target = "x86_64"
(module
  (func (;0;) (param i32)
    local.get 0
    block ;; label = @1
      i32.const 808727609
      br_table 0 (;@1;) 1 (;@0;) 0 (;@1;)
    end
    drop
  )
  (export "main" (func 0))
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8759000000         	ja	0x71
;;   18:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 b839343430           	mov	eax, 0x30343439
;;      	 b902000000           	mov	ecx, 2
;;      	 39c1                 	cmp	ecx, eax
;;      	 0f42c1               	cmovb	eax, ecx
;;      	 4c8d1d0a000000       	lea	r11, [rip + 0xa]
;;      	 49630c83             	movsxd	rcx, dword ptr [r11 + rax*4]
;;      	 4901cb               	add	r11, rcx
;;      	 41ffe3               	jmp	r11
;;   4d:	 1a00                 	sbb	al, byte ptr [rax]
;;      	 0000                 	add	byte ptr [rax], al
;;      	 1100                 	adc	dword ptr [rax], eax
;;      	 0000                 	add	byte ptr [rax], al
;;      	 1a00                 	sbb	al, byte ptr [rax]
;;      	 0000                 	add	byte ptr [rax], al
;;      	 e909000000           	jmp	0x67
;;   5e:	 4883c404             	add	rsp, 4
;;      	 e904000000           	jmp	0x6b
;;   67:	 4883c404             	add	rsp, 4
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   71:	 0f0b                 	ud2	
