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
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c893424             	mov	qword ptr [rsp], r14
;;   10:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   15:	 4883ec04             	sub	rsp, 4
;;   19:	 44891c24             	mov	dword ptr [rsp], r11d
;;   1d:	 b839343430           	mov	eax, 0x30343439
;;   22:	 b902000000           	mov	ecx, 2
;;   27:	 39c1                 	cmp	ecx, eax
;;   29:	 0f42c1               	cmovb	eax, ecx
;;   2c:	 4c8d1d0a000000       	lea	r11, [rip + 0xa]
;;   33:	 49630c83             	movsxd	rcx, dword ptr [r11 + rax*4]
;;   37:	 4901cb               	add	r11, rcx
;;   3a:	 41ffe3               	jmp	r11
;;   3d:	 1a00                 	sbb	al, byte ptr [rax]
;;   3f:	 0000                 	add	byte ptr [rax], al
;;   41:	 1100                 	adc	dword ptr [rax], eax
;;   43:	 0000                 	add	byte ptr [rax], al
;;   45:	 1a00                 	sbb	al, byte ptr [rax]
;;   47:	 0000                 	add	byte ptr [rax], al
;;   49:	 e909000000           	jmp	0x57
;;   4e:	 4883c404             	add	rsp, 4
;;   52:	 e904000000           	jmp	0x5b
;;   57:	 4883c404             	add	rsp, 4
;;   5b:	 4883c410             	add	rsp, 0x10
;;   5f:	 5d                   	pop	rbp
;;   60:	 c3                   	ret	
