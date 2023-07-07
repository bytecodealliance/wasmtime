;;! target = "x86_64"

(module
  (func $main (result i32)
    (local $var i32)
    (call $product (i32.const 20) (i32.const 80))
    (local.set $var (i32.const 2))
    (local.get $var)
    (i32.div_u))

  (func $product (param i32 i32) (result i32)
    (local.get 0)
    (local.get 1)
    (i32.mul))
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   16:	 bf14000000           	mov	edi, 0x14
;;   1b:	 be50000000           	mov	esi, 0x50
;;   20:	 e800000000           	call	0x25
;;   25:	 b902000000           	mov	ecx, 2
;;   2a:	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;   2e:	 50                   	push	rax
;;   2f:	 448b5c2414           	mov	r11d, dword ptr [rsp + 0x14]
;;   34:	 4153                 	push	r11
;;   36:	 59                   	pop	rcx
;;   37:	 58                   	pop	rax
;;   38:	 31d2                 	xor	edx, edx
;;   3a:	 f7f1                 	div	ecx
;;   3c:	 4883c410             	add	rsp, 0x10
;;   40:	 5d                   	pop	rbp
;;   41:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 89742408             	mov	dword ptr [rsp + 8], esi
;;   10:	 4c893424             	mov	qword ptr [rsp], r14
;;   14:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   18:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   1c:	 0fafc8               	imul	ecx, eax
;;   1f:	 4889c8               	mov	rax, rcx
;;   22:	 4883c410             	add	rsp, 0x10
;;   26:	 5d                   	pop	rbp
;;   27:	 c3                   	ret	
