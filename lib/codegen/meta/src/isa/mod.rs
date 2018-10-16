/// Represents known ISA target.
#[derive(Copy, Clone)]
pub enum Isa {
    Riscv,
    X86,
    Arm32,
    Arm64,
}

impl Isa {
    /// Creates isa target using name.
    pub fn new(name: &str) -> Option<Self> {
        Isa::all()
            .iter()
            .cloned()
            .filter(|isa| isa.name() == name)
            .next()
    }

    /// Creates isa target from arch.
    pub fn from_arch(arch: &str) -> Option<Isa> {
        Isa::all()
            .iter()
            .cloned()
            .filter(|isa| isa.is_arch_applicable(arch))
            .next()
    }

    /// Returns all supported isa targets.
    pub fn all() -> [Isa; 4] {
        [Isa::Riscv, Isa::X86, Isa::Arm32, Isa::Arm64]
    }

    /// Returns name of the isa target.
    pub fn name(&self) -> &'static str {
        match *self {
            Isa::Riscv => "riscv",
            Isa::X86 => "x86",
            Isa::Arm32 => "arm32",
            Isa::Arm64 => "arm64",
        }
    }

    /// Checks if arch is applicable for the isa target.
    fn is_arch_applicable(&self, arch: &str) -> bool {
        match *self {
            Isa::Riscv => arch == "riscv",
            Isa::X86 => ["x86_64", "i386", "i586", "i686"].contains(&arch),
            Isa::Arm32 => arch.starts_with("arm") || arch.starts_with("thumb"),
            Isa::Arm64 => arch == "aarch64",
        }
    }
}
