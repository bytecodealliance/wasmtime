;;! target = "x86_64"

(module
    (func (result i32)
        (local $foo i32)  
        (local $bar i32)

        (i32.const 10)
        (local.set $foo)

        (i32.const 20)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        i32.sub
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8731000000         	ja	0x49
;;   18:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b80a000000           	mov	eax, 0xa
;;      	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;      	 b814000000           	mov	eax, 0x14
;;      	 89442408             	mov	dword ptr [rsp + 8], eax
;;      	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;      	 29c1                 	sub	ecx, eax
;;      	 89c8                 	mov	eax, ecx
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   49:	 0f0b                 	ud2	
