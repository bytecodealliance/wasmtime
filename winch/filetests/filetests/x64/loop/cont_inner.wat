;;! target = "x86_64"
(module
  (func (export "cont-inner") (result i32)
    (local i32)
    (local.set 0 (i32.const 0))
    (local.set 0 (i32.add (local.get 0) (loop (result i32) (loop (result i32) (br 1)))))
    (local.set 0 (i32.add (local.get 0) (loop (result i32) (i32.ctz (br 0)))))
    (local.set 0 (i32.add (local.get 0) (loop (result i32) (i32.ctz (loop (result i32) (br 1))))))
    (local.get 0)
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c31c000000       	add	r11, 0x1c
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8737000000         	ja	0x55
;;   1e:	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;      	 4889742408           	mov	qword ptr [rsp + 8], rsi
;;      	 48c7042400000000     	mov	qword ptr [rsp], 0
;;      	 b800000000           	mov	eax, 0
;;      	 89442404             	mov	dword ptr [rsp + 4], eax
;;      	 448b5c2404           	mov	r11d, dword ptr [rsp + 4]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 e9fbffffff           	jmp	0x4a
;;   4f:	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   55:	 0f0b                 	ud2	
