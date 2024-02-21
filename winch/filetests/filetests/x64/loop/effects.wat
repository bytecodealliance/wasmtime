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
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c318000000       	add	r11, 0x18
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8756000000         	ja	0x74
;;   1e:	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;      	 4889742408           	mov	qword ptr [rsp + 8], rsi
;;      	 48c7042400000000     	mov	qword ptr [rsp], 0
;;      	 b801000000           	mov	eax, 1
;;      	 89442404             	mov	dword ptr [rsp + 4], eax
;;      	 8b442404             	mov	eax, dword ptr [rsp + 4]
;;      	 6bc003               	imul	eax, eax, 3
;;      	 89442404             	mov	dword ptr [rsp + 4], eax
;;      	 8b442404             	mov	eax, dword ptr [rsp + 4]
;;      	 83e805               	sub	eax, 5
;;      	 89442404             	mov	dword ptr [rsp + 4], eax
;;      	 8b442404             	mov	eax, dword ptr [rsp + 4]
;;      	 6bc007               	imul	eax, eax, 7
;;      	 89442404             	mov	dword ptr [rsp + 4], eax
;;      	 8b442404             	mov	eax, dword ptr [rsp + 4]
;;      	 83f8f2               	cmp	eax, -0xe
;;      	 b800000000           	mov	eax, 0
;;      	 400f94c0             	sete	al
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   74:	 0f0b                 	ud2	
