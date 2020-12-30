use std::fmt::Write;
use wasmtime::*;

pub struct WatGenerator {
    tmp: usize,
    dst: String,
}

impl WatGenerator {
    pub fn new() -> WatGenerator {
        WatGenerator {
            tmp: 0,
            dst: String::from("(module\n"),
        }
    }

    pub fn finish(mut self) -> String {
        self.dst.push_str(")\n");
        self.dst
    }

    pub fn import(&mut self, ty: &ImportType<'_>) {
        write!(self.dst, "(import ").unwrap();
        self.str(ty.module());
        write!(self.dst, " ").unwrap();
        if let Some(field) = ty.name() {
            self.str(field);
            write!(self.dst, " ").unwrap();
        }
        self.item_ty(&ty.ty());
        writeln!(self.dst, ")").unwrap();
    }

    pub fn item_ty(&mut self, ty: &ExternType) {
        match ty {
            ExternType::Memory(mem) => {
                write!(
                    self.dst,
                    "(memory {} {})",
                    mem.limits().min(),
                    match mem.limits().max() {
                        Some(max) => max.to_string(),
                        None => String::new(),
                    }
                )
                .unwrap();
            }
            ExternType::Table(table) => {
                write!(
                    self.dst,
                    "(table {} {} {})",
                    table.limits().min(),
                    match table.limits().max() {
                        Some(max) => max.to_string(),
                        None => String::new(),
                    },
                    wat_ty(table.element()),
                )
                .unwrap();
            }
            ExternType::Global(ty) => {
                if ty.mutability() == Mutability::Const {
                    write!(self.dst, "(global {})", wat_ty(ty.content())).unwrap();
                } else {
                    write!(self.dst, "(global (mut {}))", wat_ty(ty.content())).unwrap();
                }
            }
            ExternType::Func(ty) => {
                write!(self.dst, "(func ").unwrap();
                self.func_sig(ty);
                write!(self.dst, ")").unwrap();
            }
            ExternType::Instance(ty) => {
                writeln!(self.dst, "(instance").unwrap();
                for ty in ty.exports() {
                    write!(self.dst, "(export ").unwrap();
                    self.str(ty.name());
                    write!(self.dst, " ").unwrap();
                    self.item_ty(&ty.ty());
                    writeln!(self.dst, ")").unwrap();
                }
                write!(self.dst, ")").unwrap();
            }
            ExternType::Module(ty) => {
                writeln!(self.dst, "(module").unwrap();
                for ty in ty.imports() {
                    self.import(&ty);
                    writeln!(self.dst, "").unwrap();
                }
                for ty in ty.exports() {
                    write!(self.dst, "(export ").unwrap();
                    self.str(ty.name());
                    write!(self.dst, " ").unwrap();
                    self.item_ty(&ty.ty());
                    writeln!(self.dst, ")").unwrap();
                }
                write!(self.dst, ")").unwrap();
            }
        }
    }

    pub fn export(&mut self, ty: &ExportType<'_>) {
        let wat_name = format!("item{}", self.tmp);
        self.tmp += 1;
        let item_ty = ty.ty();
        self.item(&wat_name, &item_ty);

        write!(self.dst, "(export ").unwrap();
        self.str(ty.name());
        write!(self.dst, " (").unwrap();
        match item_ty {
            ExternType::Memory(_) => write!(self.dst, "memory").unwrap(),
            ExternType::Global(_) => write!(self.dst, "global").unwrap(),
            ExternType::Func(_) => write!(self.dst, "func").unwrap(),
            ExternType::Instance(_) => write!(self.dst, "instance").unwrap(),
            ExternType::Table(_) => write!(self.dst, "table").unwrap(),
            ExternType::Module(_) => write!(self.dst, "module").unwrap(),
        }
        writeln!(self.dst, " ${}))", wat_name).unwrap();
    }

    fn item(&mut self, name: &str, ty: &ExternType) {
        match ty {
            ExternType::Memory(mem) => {
                write!(
                    self.dst,
                    "(memory ${} {} {})\n",
                    name,
                    mem.limits().min(),
                    match mem.limits().max() {
                        Some(max) => max.to_string(),
                        None => String::new(),
                    }
                )
                .unwrap();
            }
            ExternType::Table(table) => {
                write!(
                    self.dst,
                    "(table ${} {} {} {})\n",
                    name,
                    table.limits().min(),
                    match table.limits().max() {
                        Some(max) => max.to_string(),
                        None => String::new(),
                    },
                    wat_ty(table.element()),
                )
                .unwrap();
            }
            ExternType::Global(ty) => {
                write!(self.dst, "(global ${} ", name).unwrap();
                if ty.mutability() == Mutability::Var {
                    write!(self.dst, "(mut ").unwrap();
                }
                write!(self.dst, "{}", wat_ty(ty.content())).unwrap();
                if ty.mutability() == Mutability::Var {
                    write!(self.dst, ")").unwrap();
                }
                write!(self.dst, " (").unwrap();
                self.value(ty.content());
                writeln!(self.dst, "))").unwrap();
            }
            ExternType::Func(ty) => {
                write!(self.dst, "(func ${} ", name).unwrap();
                self.func_sig(ty);
                for ty in ty.results() {
                    writeln!(self.dst, "").unwrap();
                    self.value(&ty);
                }
                writeln!(self.dst, ")").unwrap();
            }
            ExternType::Module(ty) => {
                writeln!(self.dst, "(module ${}", name).unwrap();
                for ty in ty.imports() {
                    self.import(&ty);
                }
                for ty in ty.exports() {
                    self.export(&ty);
                }
                self.dst.push_str(")\n");
            }
            ExternType::Instance(ty) => {
                writeln!(self.dst, "(module ${}_module", name).unwrap();
                for ty in ty.exports() {
                    self.export(&ty);
                }
                self.dst.push_str(")\n");
                writeln!(self.dst, "(instance ${} (instantiate ${0}_module))", name).unwrap();
            }
        }
    }

    fn func_sig(&mut self, ty: &FuncType) {
        write!(self.dst, "(param ").unwrap();
        for ty in ty.params() {
            write!(self.dst, "{} ", wat_ty(&ty)).unwrap();
        }
        write!(self.dst, ") (result ").unwrap();
        for ty in ty.results() {
            write!(self.dst, "{} ", wat_ty(&ty)).unwrap();
        }
        write!(self.dst, ")").unwrap();
    }

    fn value(&mut self, ty: &ValType) {
        match ty {
            ValType::I32 => write!(self.dst, "i32.const 0").unwrap(),
            ValType::I64 => write!(self.dst, "i64.const 0").unwrap(),
            ValType::F32 => write!(self.dst, "f32.const 0").unwrap(),
            ValType::F64 => write!(self.dst, "f64.const 0").unwrap(),
            ValType::V128 => write!(self.dst, "v128.const i32x4 0 0 0 0").unwrap(),
            ValType::ExternRef => write!(self.dst, "ref.null extern").unwrap(),
            ValType::FuncRef => write!(self.dst, "ref.null func").unwrap(),
        }
    }

    fn str(&mut self, name: &str) {
        let mut bytes = [0; 4];
        self.dst.push_str("\"");
        for c in name.chars() {
            let v = c as u32;
            if v >= 0x20 && v < 0x7f && c != '"' && c != '\\' && v < 0xff {
                self.dst.push(c);
            } else {
                for byte in c.encode_utf8(&mut bytes).as_bytes() {
                    self.hex_byte(*byte);
                }
            }
        }
        self.dst.push_str("\"");
    }

    fn hex_byte(&mut self, byte: u8) {
        fn to_hex(b: u8) -> char {
            if b < 10 {
                (b'0' + b) as char
            } else {
                (b'a' + b - 10) as char
            }
        }
        self.dst.push('\\');
        self.dst.push(to_hex((byte >> 4) & 0xf));
        self.dst.push(to_hex(byte & 0xf));
    }
}

fn wat_ty(ty: &ValType) -> &'static str {
    match ty {
        ValType::I32 => "i32",
        ValType::I64 => "i64",
        ValType::F32 => "f32",
        ValType::F64 => "f64",
        ValType::V128 => "v128",
        ValType::ExternRef => "externref",
        ValType::FuncRef => "funcref",
    }
}
