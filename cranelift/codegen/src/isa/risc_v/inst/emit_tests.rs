use crate::isa::risc_v::inst::*;
use crate::settings;
use alloc::vec::Vec;

/*

    todo:: more instruction

*/
#[test]
fn test_riscv64_binemit() {
    struct TestUnit {
        inst: Inst,
        assembly: &'static str,
        code: Option<u32>,
    }
    impl TestUnit {
        fn new(i: Inst, ass: &'static str) -> Self {
            Self {
                inst: i,
                assembly: ass,
                code: None,
            }
        }
    }

    let mut insns = Vec::<TestUnit>::new();
    //todo:: more
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Add,
            rd: writable_fp_reg(),
            rs1: fp_reg(),
            rs2: zero_reg(),
        },
        "add fp,fp,zero",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Addi,
            rd: writable_fp_reg(),
            rs: stack_reg(),
            imm12: Imm12::maybe_from_u64(100).unwrap(),
        },
        "addi fp,sp,100",
    ));

    {
        // generated code to speed up the test unit,otherwise you need invoke riscv-gun tool chain every time.
        // insns[0].code = Some(263219);
    }
    let flags = settings::Flags::new(settings::builder());
    let rru = create_reg_universe(&flags);
    let emit_info = EmitInfo::new(flags);
    let mut missing_code = vec![];
    for (index, ref mut unit) in insns.into_iter().enumerate() {
        println!("Riscv64: {:?}, {}", unit.inst, unit.assembly);
        // Check the printed text is as expected.
        let actual_printing = unit.inst.show_rru(Some(&rru));
        assert_eq!(unit.assembly, actual_printing);
        if unit.code.is_none() {
            let code = assemble(unit.assembly);
            missing_code.push((index, code));
            unit.code = Some(code);
        }
        let mut buffer = MachBuffer::new();
        unit.inst
            .emit(&mut buffer, &emit_info, &mut Default::default());
        let buffer = buffer.finish();
        assert_eq!(buffer.data(), unit.code.unwrap().to_le_bytes());
    }
    if missing_code.len() > 0 {
        println!("// generated code to speed up the test unit,otherwise you need invode riscv-gun tool chain every time.");
        for i in missing_code {
            println!("insns[{}].code = Some({});", i.0, i.1);
        }
        println!("");
    }
}

/*
    todo:: make this can be run on windows
*/
fn assemble(code: &str) -> u32 {
    use std::process::Command;
    std::env::set_current_dir("/var/tmp").expect("set_current_dir {}");
    let file_name = "riscv_tmp.s";
    use std::io::Write;
    let mut file = std::fs::File::create(file_name).unwrap();
    file.write_all(code.as_bytes()).expect("write error {}");
    let mut cmd = Command::new("riscv64-linux-gnu-as");
    cmd.arg(file_name);
    let _output = cmd.output().expect("exec riscv64-linux-gnu-as failed , {}");
    let output_file = "a.out";
    let mut cmd = Command::new("riscv64-linux-gnu-objdump");
    cmd.arg("-d").arg(output_file);

    let output = cmd
        .output()
        .expect("exec riscv64-linux-gnu-objdump failed , {}");
    /*
        a.out:     file format elf64-littleriscv


    Disassembly of section .text:

    0000000000000000 <.text>:
       0:   fe010113                addi    sp,sp,-32
        */
    let output = output.stdout;
    // println!(
    //     "##############################{}",
    //     String::from_iter(output.clone().into_iter().map(|c| c as char))
    // );
    // need parse this
    // right row only generate one instruction.
    // so it is easy
    for mut i in 0..output.len() {
        // match   0:
        let mut _match = || -> bool {
            if output[i] == ('0' as u8) && output[i + 1] == (':' as u8) {
                // skip 0:
                i += 2;
                true
            } else {
                false
            }
        };
        if _match() {
            // skip all white space or \t
            loop {
                if output[i] != 32 && output[i] != 9 {
                    break;
                }
                i += 1;
            }
            let mut byte_string: String = "".into();
            loop {
                if (output[i] >= ('0' as u8) && output[i] <= ('9' as u8))
                    || (output[i] >= ('a' as u8) && output[i] <= ('f' as u8))
                {
                    byte_string.push(output[i] as char);
                    i += 1;
                } else {
                    break;
                }
            }
            return u32::from_str_radix(byte_string.as_str(), 16).unwrap();
        }
    }
    unreachable!()
}
