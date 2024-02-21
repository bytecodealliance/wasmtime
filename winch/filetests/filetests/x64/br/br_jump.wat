;;! target = "x86_64"
(module
  (func (;0;) (result i32)
    (local i32)
    local.get 0
    loop ;; label = @1
      local.get 0
      block ;; label = @2
      end
      br 0 (;@1;)
    end
  )
  (export "" (func 0))
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c320000000       	add	r11, 0x20
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f873f000000         	ja	0x5d
;;   1e:	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;      	 4889742408           	mov	qword ptr [rsp + 8], rsi
;;      	 48c7042400000000     	mov	qword ptr [rsp], 0
;;      	 448b5c2404           	mov	r11d, dword ptr [rsp + 4]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 448b5c2408           	mov	r11d, dword ptr [rsp + 8]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 4883c404             	add	rsp, 4
;;      	 e9eaffffff           	jmp	0x41
;;   57:	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   5d:	 0f0b                 	ud2	
