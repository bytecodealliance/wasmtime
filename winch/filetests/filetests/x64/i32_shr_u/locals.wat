;;! target = "x86_64"

(module
    (func (result i32)
        (local $foo i32)  
        (local $bar i32)

        (i32.const 1)
        (local.set $foo)

        (i32.const 2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        (i32.shr_u)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f872f000000         	ja	0x47
;;   18:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b801000000           	mov	eax, 1
;;      	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;      	 b802000000           	mov	eax, 2
;;      	 89442408             	mov	dword ptr [rsp + 8], eax
;;      	 8b4c2408             	mov	ecx, dword ptr [rsp + 8]
;;      	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 d3e8                 	shr	eax, cl
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   47:	 0f0b                 	ud2	
