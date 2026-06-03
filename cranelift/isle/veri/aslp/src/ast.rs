#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Stmt {
    ConstDecl {
        ty: Type,
        name: String,
        rhs: Expr,
    },
    VarDecl {
        ty: Type,
        name: String,
        rhs: Expr,
    },
    VarDeclsNoInit {
        ty: Type,
        names: Vec<String>,
    },
    Assign {
        lhs: LExpr,
        rhs: Expr,
    },
    Assert {
        cond: Expr,
    },
    If {
        cond: Expr,
        then_block: Block,
        else_block: Block,
    },
    Call {
        func: Func,
        types: Vec<Expr>,
        args: Vec<Expr>,
    },
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum LExpr {
    ArrayIndex { array: Box<LExpr>, index: Box<Expr> },
    Field { x: Box<LExpr>, name: String },
    Var(String),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Expr {
    Apply {
        func: Func,
        types: Vec<Expr>,
        args: Vec<Expr>,
    },
    ArrayIndex {
        array: Box<Expr>,
        index: Box<Expr>,
    },
    Field {
        x: Box<Expr>,
        name: String,
    },
    Slices {
        x: Box<Expr>,
        slices: Vec<Slice>,
    },
    Var(String),
    LitInt(String),
    LitBits(String),
}

impl Expr {
    pub fn as_lit_int(&self) -> Option<&String> {
        match self {
            Expr::LitInt(i) => Some(i),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Slice {
    LowWidth(Box<Expr>, Box<Expr>),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Func {
    pub name: String,
    pub id: usize,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Type {
    Bits(Box<Expr>),
    Bool,
}
