use crate::constraints::Target;

pub struct ReadEffect {
    pub active: Target,
    pub addr: Target,
    pub size_bits: Target,
    pub value: Target,
}

impl ReadEffect {
    pub fn new() -> Self {
        Self {
            active: Target::Field(Box::new(Self::variable()), "ACTIVE".to_string()),
            addr: Target::Field(Box::new(Self::variable()), "ADDR".to_string()),
            size_bits: Target::Field(Box::new(Self::variable()), "SIZE_BITS".to_string()),
            value: Target::Field(Box::new(Self::variable()), "VALUE".to_string()),
        }
    }

    fn variable() -> Target {
        Target::Var("MEMORY_READ_EFFECT".to_string())
    }

    pub fn targets(&self) -> Vec<&Target> {
        vec![&self.active, &self.addr, &self.size_bits, &self.value]
    }
}

pub struct SetEffect {
    pub active: Target,
    pub addr: Target,
    pub size_bits: Target,
    pub value: Target,
}

impl SetEffect {
    pub fn new() -> Self {
        Self {
            active: Target::Field(Box::new(Self::variable()), "ACTIVE".to_string()),
            addr: Target::Field(Box::new(Self::variable()), "ADDR".to_string()),
            size_bits: Target::Field(Box::new(Self::variable()), "SIZE_BITS".to_string()),
            value: Target::Field(Box::new(Self::variable()), "VALUE".to_string()),
        }
    }

    fn variable() -> Target {
        Target::Var("MEMORY_SET_EFFECT".to_string())
    }

    pub fn targets(&self) -> Vec<&Target> {
        vec![&self.active, &self.addr, &self.size_bits, &self.value]
    }
}
