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
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 bf14000000           	mov	edi, 0x14
;;   1a:	 be50000000           	mov	esi, 0x50
;;   1f:	 e800000000           	call	0x24
;;   24:	 b902000000           	mov	ecx, 2
;;   29:	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;   2d:	 50                   	push	rax
;;   2e:	 448b5c2414           	mov	r11d, dword ptr [rsp + 0x14]
;;   33:	 4153                 	push	r11
;;   35:	 59                   	pop	rcx
;;   36:	 58                   	pop	rax
;;   37:	 31d2                 	xor	edx, edx
;;   39:	 f7f1                 	div	ecx
;;   3b:	 4883c410             	add	rsp, 0x10
;;   3f:	 5d                   	pop	rbp
;;   40:	 c3                   	ret	
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
;;   1f:	 89c8                 	mov	eax, ecx
;;   21:	 4883c410             	add	rsp, 0x10
;;   25:	 5d                   	pop	rbp
;;   26:	 c3                   	ret	
