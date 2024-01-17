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
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8751000000         	ja	0x69
;;   18:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 bf14000000           	mov	edi, 0x14
;;      	 be50000000           	mov	esi, 0x50
;;      	 e800000000           	call	0x34
;;      	 b902000000           	mov	ecx, 2
;;      	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 448b5c2410           	mov	r11d, dword ptr [rsp + 0x10]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 8b0c24               	mov	ecx, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 8b0424               	mov	eax, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 31d2                 	xor	edx, edx
;;      	 f7f1                 	div	ecx
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   69:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f871f000000         	ja	0x37
;;   18:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 89742408             	mov	dword ptr [rsp + 8], esi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;      	 0fafc8               	imul	ecx, eax
;;      	 89c8                 	mov	eax, ecx
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   37:	 0f0b                 	ud2	
