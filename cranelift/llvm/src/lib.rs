use std::collections::HashMap;

use cranelift_codegen::ir::{ExternalName, LibCall, Signature};
use cranelift_codegen::isa::TargetIsa;
use cranelift_module::{DataContext, DataId, FuncId, ModuleCompiledFunction, ModuleDeclarations};
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::memory_buffer::MemoryBuffer;
use inkwell::module::Module;
use inkwell::passes::PassManager;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetTriple,
};
use inkwell::types::{BasicTypeEnum, FunctionType, StructType};
use inkwell::values::{BasicValueEnum, FunctionValue, GlobalValue};
use inkwell::OptimizationLevel;

mod function;

pub struct LlvmModule<'ctx> {
    isa: &'ctx dyn TargetIsa,
    declarations: ModuleDeclarations,

    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,

    intrinsic_refs: HashMap<String, FunctionValue<'ctx>>,
    libcall_refs: HashMap<LibCall, FunctionValue<'ctx>>,
    function_refs: HashMap<FuncId, FunctionValue<'ctx>>,
    data_object_refs: HashMap<DataId, GlobalValue<'ctx>>,
    data_object_types: HashMap<DataId, StructType<'ctx>>,
}

impl<'ctx> LlvmModule<'ctx> {
    pub fn with_module<T>(
        name: &str,
        isa: &dyn TargetIsa,
        f: impl for<'a> FnOnce(&mut LlvmModule<'a>) -> T,
    ) -> T {
        let context = Context::create();
        let x = f(&mut LlvmModule {
            isa,
            declarations: ModuleDeclarations::default(),
            context: &context,
            module: context.create_module(name),
            builder: context.create_builder(),

            intrinsic_refs: HashMap::new(),
            libcall_refs: HashMap::new(),
            function_refs: HashMap::new(),
            data_object_refs: HashMap::new(),
            data_object_types: HashMap::new(),
        });
        x
    }

    pub fn print_to_stderr(&self) {
        self.module.print_to_stderr();
    }

    pub fn compile(&mut self) -> MemoryBuffer {
        //self.print_to_stderr();

        if let Err(err) = self.module.verify() {
            println!("{}", err.to_string_lossy());
            panic!();
        }

        let pass_manager: PassManager<Module> = PassManager::create(());
        pass_manager.add_instruction_combining_pass();
        pass_manager.add_reassociate_pass();
        pass_manager.add_gvn_pass();
        pass_manager.add_cfg_simplification_pass();
        pass_manager.add_basic_alias_analysis_pass();
        pass_manager.add_promote_memory_to_register_pass();
        pass_manager.add_instruction_combining_pass();
        pass_manager.add_reassociate_pass();
        pass_manager.run_on(&self.module);
        //self.print_to_stderr();
        self.module.print_to_file(format!("/tmp/{}.ll", self.module.get_name().to_str().unwrap())).unwrap();


        Target::initialize_x86(&InitializationConfig::default());
        let opt = OptimizationLevel::Default;
        let target = Target::from_name("x86-64").unwrap();
        let target_machine = target
            .create_target_machine(
                &TargetTriple::create("x86_64-pc-linux-gnu"),
                "x86-64",
                "+avx2",
                opt,
                RelocMode::PIC,
                CodeModel::Default,
            )
            .unwrap();

        target_machine.write_to_memory_buffer(&self.module, FileType::Object).unwrap()
    }

    fn get_intrinsic(&mut self, name: String, ty: FunctionType<'ctx>) -> FunctionValue<'ctx> {
        *self
            .intrinsic_refs
            .entry(name.clone())
            .or_insert_with(|| self.module.add_function(&name, ty, None))
    }

    fn get_func(&mut self, ext_name: &ExternalName) -> FunctionValue<'ctx> {
        match ext_name {
            ExternalName::User { .. } => {
                let func_id = FuncId::from_name(ext_name);
                self.function_refs[&func_id]
            }
            ExternalName::LibCall(libcall) => {
                let i8p_ty = self.context.i64_type(); //self.context.i8_type().ptr_type(inkwell::AddressSpace::Generic);
                let c_int_ty = self.context.i32_type().into(); // FIXME
                let size_t_ty = self.context.i64_type().into(); // FIXME
                let (name, ty) = match *libcall {
                    LibCall::Memcpy => (
                        "memcpy",
                        i8p_ty.fn_type(&[i8p_ty.into(), i8p_ty.into(), size_t_ty], false),
                    ),
                    LibCall::Memset => {
                        ("memset", i8p_ty.fn_type(&[i8p_ty.into(), c_int_ty, size_t_ty], false))
                    }
                    LibCall::Memmove => (
                        "memmove",
                        i8p_ty.fn_type(&[i8p_ty.into(), i8p_ty.into(), size_t_ty], false),
                    ),
                    LibCall::UdivI64 => todo!(),
                    LibCall::SdivI64 => todo!(),
                    LibCall::UremI64 => todo!(),
                    LibCall::SremI64 => todo!(),
                    LibCall::IshlI64 => todo!(),
                    LibCall::UshrI64 => todo!(),
                    LibCall::SshrI64 => todo!(),
                    LibCall::CeilF32 => todo!(),
                    LibCall::CeilF64 => todo!(),
                    LibCall::FloorF32 => todo!(),
                    LibCall::FloorF64 => todo!(),
                    LibCall::TruncF32 => todo!(),
                    LibCall::TruncF64 => todo!(),
                    LibCall::NearestF32 => todo!(),
                    LibCall::NearestF64 => todo!(),
                    LibCall::Probestack | LibCall::ElfTlsGetAddr => todo!(),
                };
                *self.libcall_refs.entry(*libcall).or_insert_with(|| {
                    self.module
                        .get_function(name)
                        .unwrap_or_else(|| self.module.add_function(name, ty, None))
                })
            }
            ExternalName::TestCase { .. } => unimplemented!(),
        }
    }
}

fn translate_linkage(linkage: cranelift_module::Linkage) -> inkwell::module::Linkage {
    match linkage {
        cranelift_module::Linkage::Import => inkwell::module::Linkage::External,
        cranelift_module::Linkage::Local => inkwell::module::Linkage::Internal,
        cranelift_module::Linkage::Preemptible => inkwell::module::Linkage::ExternalWeak,
        cranelift_module::Linkage::Hidden => {
            // FIXME set hidden visibility
            inkwell::module::Linkage::External
        }
        cranelift_module::Linkage::Export => inkwell::module::Linkage::External,
    }
}

impl<'ctx> cranelift_module::Module for LlvmModule<'ctx> {
    fn isa(&self) -> &dyn cranelift_codegen::isa::TargetIsa {
        self.isa
    }

    fn declarations(&self) -> &cranelift_module::ModuleDeclarations {
        &self.declarations
    }

    fn declare_function(
        &mut self,
        name: &str,
        linkage: cranelift_module::Linkage,
        signature: &Signature,
    ) -> cranelift_module::ModuleResult<FuncId> {
        let (func_id, linkage) = self.declarations.declare_function(name, linkage, signature)?;

        let func_val = self.function_refs.entry(func_id).or_insert_with(|| {
            let func_ty = function::translate_sig(
                self.context,
                signature,
                // FIXME hack to make common variadic functions work
                name == "printf"
                    || name == "syscall"
                    || name == "fcntl"
                    || name == "ioctl"
                    || name == "prctl"
                    || name == "open"
                    || name == "open64",
            );
            if let Some(func_val) = self.module.get_function(name) {
                assert_eq!(func_ty, func_val.get_type());
                return func_val;
            }
            let func_val = self.module.add_function(name, func_ty, None);
            // FIXME apply param attributes
            func_val
        });
        func_val.set_linkage(translate_linkage(linkage));

        Ok(func_id)
    }

    fn declare_anonymous_function(
        &mut self,
        signature: &Signature,
    ) -> cranelift_module::ModuleResult<FuncId> {
        let func_id = self.declarations.declare_anonymous_function(signature)?;

        let func_val = self.module.add_function(
            &self.declarations.get_function_decl(func_id).name,
            function::translate_sig(self.context, signature, false),
            Some(inkwell::module::Linkage::Internal),
        );
        func_val.set_linkage(inkwell::module::Linkage::Internal);
        // FIXME apply param attributes
        self.function_refs.insert(func_id, func_val);

        Ok(func_id)
    }

    fn declare_data(
        &mut self,
        name: &str,
        linkage: cranelift_module::Linkage,
        writable: bool,
        tls: bool,
    ) -> cranelift_module::ModuleResult<DataId> {
        let (data_id, linkage) = self.declarations.declare_data(name, linkage, writable, tls)?;

        let data_val = self.data_object_refs.entry(data_id).or_insert_with(|| {
            let data_type = self.context.opaque_struct_type(&format!("{}_t", data_id));
            let data_val = self.module.add_global(data_type, None, name);
            data_val.set_externally_initialized(true); // Will be set to false when actually defining it
            data_val.set_constant(!writable);
            data_val.set_thread_local(tls);
            self.data_object_types.insert(data_id, data_type);
            data_val
        });
        data_val.set_linkage(translate_linkage(linkage));

        Ok(data_id)
    }

    fn declare_anonymous_data(
        &mut self,
        writable: bool,
        tls: bool,
    ) -> cranelift_module::ModuleResult<DataId> {
        let data_id = self.declarations.declare_anonymous_data(writable, tls)?;

        let data_type = self.context.opaque_struct_type(&format!("{}_t", data_id));
        let data_val =
            self.module.add_global(data_type, None, &self.declarations.get_data_decl(data_id).name);
        data_val.set_constant(!writable);
        data_val.set_thread_local(tls);
        data_val.set_linkage(inkwell::module::Linkage::Internal);
        self.data_object_refs.insert(data_id, data_val);
        self.data_object_types.insert(data_id, data_type);

        Ok(data_id)
    }

    fn define_function(
        &mut self,
        func_id: FuncId,
        ctx: &mut cranelift_codegen::Context,
        _trap_sink: &mut dyn cranelift_codegen::binemit::TrapSink,
        _stack_map_sink: &mut dyn cranelift_codegen::binemit::StackMapSink,
    ) -> cranelift_module::ModuleResult<cranelift_module::ModuleCompiledFunction> {
        function::define_function(self, func_id, ctx)?;

        Ok(ModuleCompiledFunction { size: 0 })
    }

    fn define_function_bytes(
        &mut self,
        _func: FuncId,
        _bytes: &[u8],
        _relocs: &[cranelift_module::RelocRecord],
    ) -> cranelift_module::ModuleResult<cranelift_module::ModuleCompiledFunction> {
        unimplemented!("define_function_bytes can't be implemented for LLVM");
    }

    fn define_data(
        &mut self,
        data_id: DataId,
        data_ctx: &DataContext,
    ) -> cranelift_module::ModuleResult<()> {
        self.data_object_refs[&data_id].set_externally_initialized(false);

        if data_ctx.description().function_relocs.is_empty()
            && data_ctx.description().data_relocs.is_empty()
        {
            self.data_object_types[&data_id].set_body(
                &[self
                    .context
                    .i8_type()
                    .array_type(data_ctx.description().init.size().try_into().unwrap())
                    .into()],
                true,
            );

            let bytes = match data_ctx.description().init {
                cranelift_module::Init::Zeros { size } => {
                    self.context.i8_type().array_type(size.try_into().unwrap()).const_zero()
                }
                cranelift_module::Init::Bytes { ref contents } => {
                    self.context.i8_type().const_array(
                        &contents
                            .iter()
                            .map(|&byte| self.context.i8_type().const_int(byte.into(), false))
                            .collect::<Vec<_>>(),
                    )
                }
                cranelift_module::Init::Uninitialized => unreachable!(),
            };

            self.data_object_refs[&data_id].set_initializer(
                &self.data_object_types[&data_id].const_named_struct(&[bytes.into()]),
            );
        } else {
            let ptr_size = 8; // FIXME
            let ptr_ty = self.context.i64_type(); // FIXME

            let mut relocs: HashMap<u32, BasicValueEnum> = HashMap::new();
            for &(offset, func_ref) in &data_ctx.description().function_relocs {
                relocs.insert(
                    offset,
                    self.get_func(&data_ctx.description().function_decls[func_ref])
                        .as_global_value()
                        .as_pointer_value()
                        .const_to_int(ptr_ty)
                        .into(),
                );
            }
            for &(offset, data_ref, addend) in &data_ctx.description().data_relocs {
                relocs.insert(
                    offset,
                    self.data_object_refs
                        [&DataId::from_name(&data_ctx.description().data_decls[data_ref])]
                        .as_pointer_value()
                        .const_to_int(ptr_ty)
                        .const_add(ptr_ty.const_int(addend as u64, false))
                        .into(),
                );
            }

            let mut type_parts: Vec<BasicTypeEnum> = vec![];
            let mut data_parts: Vec<BasicValueEnum> = vec![];

            macro_rules! push_byte_range {
                ($range:expr) => {
                    if !$range.is_empty() {
                        let ty =
                            self.context.i8_type().array_type($range.len().try_into().unwrap());
                        type_parts.push(ty.into());
                        data_parts.push(match data_ctx.description().init {
                            cranelift_module::Init::Zeros { size: _ } => ty.const_zero().into(),
                            cranelift_module::Init::Bytes { ref contents } => self
                                .context
                                .i8_type()
                                .const_array(
                                    &contents[$range]
                                        .iter()
                                        .map(|&byte| {
                                            self.context.i8_type().const_int(byte.into(), false)
                                        })
                                        .collect::<Vec<_>>(),
                                )
                                .into(),
                            cranelift_module::Init::Uninitialized => unreachable!(),
                        });
                    }
                };
            }

            let mut i = 0;
            let size = data_ctx.description().init.size().try_into().unwrap();
            let mut current_run = 0;
            while i < size {
                if let Some(reloc) = relocs.get(&i) {
                    push_byte_range!(current_run..i as usize);
                    type_parts.push(ptr_ty.into());
                    data_parts.push(reloc.clone());
                    i += ptr_size;
                    current_run = i as usize;
                } else {
                    i += 1;
                }
            }
            push_byte_range!(current_run..i as usize);

            self.data_object_types[&data_id].set_body(&type_parts, true);
            self.data_object_refs[&data_id]
                .set_initializer(&self.data_object_types[&data_id].const_named_struct(&data_parts));
        }

        Ok(())
    }
}
