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
;;    c:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   11:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   16:	 4153                 	push	r11
;;   18:	 b839343430           	mov	eax, 0x30343439
;;   1d:	 b902000000           	mov	ecx, 2
;;   22:	 39c1                 	cmp	ecx, eax
;;   24:	 0f42c1               	cmovb	eax, ecx
;;   27:	 4c8d1d0a000000       	lea	r11, [rip + 0xa]
;;   2e:	 49630c83             	movsxd	rcx, dword ptr [r11 + rax*4]
;;   32:	 4901cb               	add	r11, rcx
;;   35:	 41ffe3               	jmp	r11
;;   38:	 0c00                 	or	al, 0
;;   3a:	 0000                 	add	byte ptr [rax], al
;;   3c:	 1000                 	adc	byte ptr [rax], al
;;   3e:	 0000                 	add	byte ptr [rax], al
;;   40:	 0c00                 	or	al, 0
;;   42:	 0000                 	add	byte ptr [rax], al
;;   44:	 4883c408             	add	rsp, 8
;;   48:	 4883c410             	add	rsp, 0x10
;;   4c:	 5d                   	pop	rbp
;;   4d:	 c3                   	ret	
