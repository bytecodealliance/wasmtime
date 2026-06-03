use anyhow::Result;
use enquote::unquote;
use pest::{
    iterators::{Pair, Pairs},
    Parser,
};
use pest_derive::Parser;
use tracing::debug;

use crate::ast::{Block, Expr, Func, LExpr, Slice, Stmt, Type};

#[derive(Parser)]
#[grammar = "aslt.pest"]
struct ASLTParser;

pub fn parse(src: &str) -> Result<Block> {
    let pairs = ASLTParser::parse(Rule::aslt, src)?;
    parse_block(pairs)
}

fn parse_block(pairs: Pairs<Rule>) -> Result<Block> {
    let stmts = parse_stmts(pairs)?;
    Ok(Block { stmts })
}

fn parse_stmts(pairs: Pairs<Rule>) -> Result<Vec<Stmt>> {
    let mut stmts = Vec::new();
    for pair in pairs {
        let rule = pair.as_rule();
        debug!(?rule, "parse stmts");
        match rule {
            Rule::stmt => stmts.push(parse_stmt(pair)?),
            Rule::EOI => break,
            _ => unreachable!("unexpected statement: {pair}"),
        }
    }
    Ok(stmts)
}

fn parse_stmt(pair: Pair<Rule>) -> Result<Stmt> {
    let rule = pair.as_rule();
    debug!(?rule, "parse stmt");
    match rule {
        Rule::stmt => parse_stmt(pair.into_inner().next().unwrap()),
        Rule::stmt_assign => {
            let mut pairs = pair.into_inner();
            let lhs = parse_lexpr(pairs.next().unwrap())?;
            let rhs = parse_expr(pairs.next().unwrap())?;
            Ok(Stmt::Assign { lhs, rhs })
        }
        Rule::stmt_constdecl => {
            let mut pairs = pair.into_inner();
            let ty = parse_type(pairs.next().unwrap())?;
            let name = parse_ident(pairs.next().unwrap())?;
            let rhs = parse_expr(pairs.next().unwrap())?;
            Ok(Stmt::ConstDecl { ty, name, rhs })
        }
        Rule::stmt_vardecl => {
            let mut pairs = pair.into_inner();
            let ty = parse_type(pairs.next().unwrap())?;
            let name = parse_ident(pairs.next().unwrap())?;
            let rhs = parse_expr(pairs.next().unwrap())?;
            Ok(Stmt::VarDecl { ty, name, rhs })
        }
        Rule::stmt_vardeclsnoinit => {
            let mut pairs = pair.into_inner();
            let ty = parse_type(pairs.next().unwrap())?;
            let names = parse_vars(pairs.next().unwrap().into_inner())?;
            Ok(Stmt::VarDeclsNoInit { ty, names })
        }
        Rule::stmt_assert => {
            let cond = parse_expr(pair.into_inner().next().unwrap())?;
            Ok(Stmt::Assert { cond })
        }
        Rule::stmt_if => {
            let mut pairs = pair.into_inner();
            let cond = parse_expr(pairs.next().unwrap())?;
            let then_block = parse_block(pairs.next().unwrap().into_inner())?;
            let elseif_block = parse_block(pairs.next().unwrap().into_inner())?;
            if !elseif_block.stmts.is_empty() {
                todo!("else if");
            }
            let else_block = parse_block(pairs.next().unwrap().into_inner())?;
            Ok(Stmt::If {
                cond,
                then_block,
                else_block,
            })
        }
        Rule::stmt_tcall => {
            let mut pairs = pair.into_inner();
            let func = parse_func_ident(pairs.next().unwrap())?;
            let types = parse_exprs(pairs.next().unwrap().into_inner())?;
            let args = parse_exprs(pairs.next().unwrap().into_inner())?;
            Ok(Stmt::Call { func, types, args })
        }
        _ => unreachable!("unexpected statement: {rule:?}"),
    }
}

fn parse_lexpr(pair: Pair<Rule>) -> Result<LExpr> {
    let rule = pair.as_rule();
    debug!(?rule, "parse lexpr");
    match rule {
        Rule::lexpr => parse_lexpr(pair.into_inner().next().unwrap()),
        Rule::lexpr_array => {
            let mut pairs = pair.into_inner();
            let array = Box::new(parse_lexpr(pairs.next().unwrap())?);
            let index = Box::new(parse_expr(pairs.next().unwrap())?);
            Ok(LExpr::ArrayIndex { array, index })
        }
        Rule::lexpr_field => {
            let mut pairs = pair.into_inner();
            let x = Box::new(parse_lexpr(pairs.next().unwrap())?);
            let name = parse_ident(pairs.next().unwrap())?;
            Ok(LExpr::Field { x, name })
        }
        Rule::lexpr_var => {
            let var = parse_var(pair.into_inner().next().unwrap())?;
            Ok(LExpr::Var(var))
        }
        _ => unreachable!("unexpected lexpr: {rule:?}"),
    }
}

fn parse_expr(pair: Pair<Rule>) -> Result<Expr> {
    let rule = pair.as_rule();
    debug!(?rule, "parse expr");
    match rule {
        Rule::expr => parse_expr(pair.into_inner().next().unwrap()),
        Rule::expr_array => {
            let mut pairs = pair.into_inner();
            let array = Box::new(parse_expr(pairs.next().unwrap())?);
            let index = Box::new(parse_expr(pairs.next().unwrap())?);
            Ok(Expr::ArrayIndex { array, index })
        }
        Rule::expr_tapply => {
            let mut pairs = pair.into_inner();
            let func = parse_func_ident(pairs.next().unwrap())?;
            let types = parse_exprs(pairs.next().unwrap().into_inner())?;
            let args = parse_exprs(pairs.next().unwrap().into_inner())?;
            Ok(Expr::Apply { func, types, args })
        }
        Rule::expr_slices => {
            let mut pairs = pair.into_inner();
            let x = Box::new(parse_expr(pairs.next().unwrap())?);
            let slices = parse_slices(pairs.next().unwrap().into_inner())?;
            Ok(Expr::Slices { x, slices })
        }
        Rule::expr_field => {
            let mut pairs = pair.into_inner();
            let x = Box::new(parse_expr(pairs.next().unwrap())?);
            let name = parse_ident(pairs.next().unwrap())?;
            Ok(Expr::Field { x, name })
        }
        Rule::expr_var => {
            let var = parse_var(pair.into_inner().next().unwrap())?;
            Ok(Expr::Var(var))
        }
        Rule::expr_litint => {
            let digits = parse_literal(pair.into_inner().next().unwrap())?;
            Ok(Expr::LitInt(digits))
        }
        Rule::expr_litbits => {
            let bits = parse_literal(pair.into_inner().next().unwrap())?;
            Ok(Expr::LitBits(bits))
        }
        _ => unreachable!("unexpected expr: {rule:?}"),
    }
}

fn parse_exprs(pairs: Pairs<Rule>) -> Result<Vec<Expr>> {
    let mut exprs = Vec::new();
    for pair in pairs {
        let rule = pair.as_rule();
        debug!(?rule, "parse exprs");
        match rule {
            Rule::expr => exprs.push(parse_expr(pair)?),
            _ => unreachable!("unexpected expression: {rule:?}"),
        }
    }
    Ok(exprs)
}

fn parse_slice(pair: Pair<Rule>) -> Result<Slice> {
    let rule = pair.as_rule();
    debug!(?rule, "parse slice");
    match rule {
        Rule::slice => parse_slice(pair.into_inner().next().unwrap()),
        Rule::slice_lowd => {
            let mut pairs = pair.into_inner();
            let low = Box::new(parse_expr(pairs.next().unwrap())?);
            let width = Box::new(parse_expr(pairs.next().unwrap())?);
            Ok(Slice::LowWidth(low, width))
        }
        _ => unreachable!("unexpected slice: {rule:?}"),
    }
}

fn parse_slices(pairs: Pairs<Rule>) -> Result<Vec<Slice>> {
    let mut slices = Vec::new();
    for pair in pairs {
        let rule = pair.as_rule();
        debug!(?rule, "parse slices");
        match rule {
            Rule::slice => slices.push(parse_slice(pair)?),
            _ => unreachable!("unexpected slice: {rule:?}"),
        }
    }
    Ok(slices)
}

fn parse_type(pair: Pair<Rule>) -> Result<Type> {
    let rule = pair.as_rule();
    debug!(?rule, "parse type");
    match rule {
        Rule::ty => parse_type(pair.into_inner().next().unwrap()),
        Rule::ty_bits => {
            let width = Box::new(parse_expr(pair.into_inner().next().unwrap())?);
            Ok(Type::Bits(width))
        }
        Rule::ty_boolean => Ok(Type::Bool),
        _ => unreachable!("unexpected type: {rule:?}"),
    }
}

fn parse_var(pair: Pair<Rule>) -> Result<String> {
    let rule = pair.as_rule();
    debug!(?rule, "parse var");
    match rule {
        Rule::var => parse_var(pair.into_inner().next().unwrap()),
        Rule::var_ident => parse_ident(pair),
        _ => unreachable!("unexpected var: {rule:?}"),
    }
}

fn parse_vars(pairs: Pairs<Rule>) -> Result<Vec<String>> {
    let mut vars = Vec::new();
    for pair in pairs {
        let rule = pair.as_rule();
        debug!(?rule, "parse vars");
        match rule {
            Rule::var => vars.push(parse_var(pair)?),
            _ => unreachable!("unexpected var: {rule:?}"),
        }
    }
    Ok(vars)
}

fn parse_func_ident(pair: Pair<Rule>) -> Result<Func> {
    let rule = pair.as_rule();
    debug!(?rule, "parse func ident");
    match rule {
        Rule::func_ident => {
            let mut pairs = pair.into_inner();
            let name = pairs.next().unwrap().as_str().to_string();
            let id = pairs.next().unwrap().as_str().parse()?;
            Ok(Func { name, id })
        }
        _ => unreachable!("unexpected func ident: {rule:?}"),
    }
}

fn parse_ident(pair: Pair<Rule>) -> Result<String> {
    Ok(unquote(pair.as_str())?)
}

fn parse_literal(pair: Pair<Rule>) -> Result<String> {
    let rule = pair.as_rule();
    debug!(?rule, "parse literal");
    match rule {
        Rule::integer => Ok(pair.as_str().to_string()),
        Rule::bits => Ok(unquote(pair.as_str())?),
        _ => unreachable!("unexpected literal: {rule:?}"),
    }
}
