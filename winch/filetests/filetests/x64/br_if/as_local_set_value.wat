;;! target = "x86_64"
(module
  (func (export "as-local-set-value") (param i32) (result i32)
    (local i32)
    (block (result i32)
      (local.set 0 (br_if 0 (i32.const 17) (local.get 0)))
      (i32.const -1)
    )
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c318000000       	add	r11, 0x18
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f873c000000         	ja	0x5a
;;   1e:	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;      	 4889742408           	mov	qword ptr [rsp + 8], rsi
;;      	 89542404             	mov	dword ptr [rsp + 4], edx
;;      	 c7042400000000       	mov	dword ptr [rsp], 0
;;      	 4531db               	xor	r11d, r11d
;;      	 8b4c2404             	mov	ecx, dword ptr [rsp + 4]
;;      	 b811000000           	mov	eax, 0x11
;;      	 85c9                 	test	ecx, ecx
;;      	 0f8509000000         	jne	0x54
;;   4b:	 89442404             	mov	dword ptr [rsp + 4], eax
;;      	 b8ffffffff           	mov	eax, 0xffffffff
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   5a:	 0f0b                 	ud2	
