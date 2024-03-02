;;! target = "x86_64"
(module
  (func $fx (export "effects") (result i32)
    (local i32)
    (block
      (loop
        (local.set 0 (i32.const 1))
        (local.set 0 (i32.mul (local.get 0) (i32.const 3)))
        (local.set 0 (i32.sub (local.get 0) (i32.const 5)))
        (local.set 0 (i32.mul (local.get 0) (i32.const 7)))
        (br 1)
        (local.set 0 (i32.mul (local.get 0) (i32.const 100)))
      )
    )
    (i32.eq (local.get 0) (i32.const -14))
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4c8b5f08             	mov	r11, qword ptr [rdi + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c348000000       	add	r11, 0x48
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8792000000         	ja	0xad
;;   1b:	 4883ec30             	sub	rsp, 0x30
;;      	 48891c24             	mov	qword ptr [rsp], rbx
;;      	 4c89642408           	mov	qword ptr [rsp + 8], r12
;;      	 4c896c2410           	mov	qword ptr [rsp + 0x10], r13
;;      	 4c89742418           	mov	qword ptr [rsp + 0x18], r14
;;      	 4c897c2420           	mov	qword ptr [rsp + 0x20], r15
;;      	 4989fe               	mov	r14, rdi
;;      	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2440           	mov	qword ptr [rsp + 0x40], rdi
;;      	 4889742438           	mov	qword ptr [rsp + 0x38], rsi
;;      	 48c744243000000000   	
;; 				mov	qword ptr [rsp + 0x30], 0
;;      	 b801000000           	mov	eax, 1
;;      	 89442434             	mov	dword ptr [rsp + 0x34], eax
;;      	 8b442434             	mov	eax, dword ptr [rsp + 0x34]
;;      	 6bc003               	imul	eax, eax, 3
;;      	 89442434             	mov	dword ptr [rsp + 0x34], eax
;;      	 8b442434             	mov	eax, dword ptr [rsp + 0x34]
;;      	 83e805               	sub	eax, 5
;;      	 89442434             	mov	dword ptr [rsp + 0x34], eax
;;      	 8b442434             	mov	eax, dword ptr [rsp + 0x34]
;;      	 6bc007               	imul	eax, eax, 7
;;      	 89442434             	mov	dword ptr [rsp + 0x34], eax
;;      	 8b442434             	mov	eax, dword ptr [rsp + 0x34]
;;      	 83f8f2               	cmp	eax, -0xe
;;      	 b800000000           	mov	eax, 0
;;      	 400f94c0             	sete	al
;;      	 4883c418             	add	rsp, 0x18
;;      	 488b1c24             	mov	rbx, qword ptr [rsp]
;;      	 4c8b642408           	mov	r12, qword ptr [rsp + 8]
;;      	 4c8b6c2410           	mov	r13, qword ptr [rsp + 0x10]
;;      	 4c8b742418           	mov	r14, qword ptr [rsp + 0x18]
;;      	 4c8b7c2420           	mov	r15, qword ptr [rsp + 0x20]
;;      	 4883c430             	add	rsp, 0x30
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   ad:	 0f0b                 	ud2	
