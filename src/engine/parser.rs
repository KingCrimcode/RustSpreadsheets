use std::{collections::HashMap, fmt};

use pest::{iterators::Pairs, pratt_parser::PrattParser, Parser};
use pest_derive::Parser;

#[derive(Debug, PartialEq)]
pub enum FormulaError {
    ParsingError,
    DivBy0,
    UnknownFunction,
}

impl fmt::Display for FormulaError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FormulaError::ParsingError => write!(f, "#NAME?"),
            FormulaError::DivBy0 => write!(f, "#DIV/0!"),
            FormulaError::UnknownFunction => write!(f, "#NAME?"),
        }
    }
}

#[derive(Parser)]
#[grammar = "engine/cell_formula.pest"]
struct CellFormulaParser;

type SpreadsheetFunction = fn(&[f64]) -> Result<f64, FormulaError>;

lazy_static::lazy_static! {
    static ref PRATT_PARSER: PrattParser<Rule> = {
        use pest::pratt_parser::{Assoc::*, Op};
        use Rule::*;
        PrattParser::new()
            .op(Op::infix(add, Left) | Op::infix(sub, Left))
            .op(Op::infix(mul, Left) | Op::infix(div, Left))
            .op(Op::infix(pow, Right))
            .op(Op::prefix(neg))
    };
    static ref FUNCTION_REGISTRY: HashMap<&'static str, SpreadsheetFunction> = {
        let mut m = HashMap::new();
        m.insert("SUM", sum as SpreadsheetFunction);
        m.insert("AVG", avg as SpreadsheetFunction);
        m
    };
}

fn sum(args: &[f64]) -> Result<f64, FormulaError> {
    Ok(args.iter().sum())
}

fn avg(args: &[f64]) -> Result<f64, FormulaError> {
    if args.is_empty() {
        return Err(FormulaError::DivBy0);
    }
    Ok(args.iter().sum::<f64>() / args.len() as f64)
}

#[derive(Debug)]
enum Expr {
    Number(f64),
    CellRef(String),
    // Range(String, String),
    BinaryOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },

    UnaryOp {
        op: UnOp,
        operand: Box<Expr>,
    },

    Function {
        name: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

#[derive(Debug)]
enum UnOp {
    Neg,
}

fn parse_expr(pairs: Pairs<Rule>) -> Expr {
    PRATT_PARSER
        .map_primary(|primary| match primary.as_rule() {
            Rule::number => Expr::Number(primary.as_str().parse().unwrap()),
            Rule::cell_ref => Expr::CellRef(primary.as_str().to_string()),
            Rule::func => {
                let mut inner = primary.into_inner();
                let name = inner.next().unwrap().as_str().to_string();
                let Some(args) = inner.next() else {
                    return Expr::Function { name, args: vec![] };
                };
                let args: Vec<Expr> = args
                    .into_inner()
                    .map(|arg| parse_expr(arg.into_inner()))
                    .collect();
                Expr::Function { name, args }
            }
            Rule::expr => parse_expr(primary.into_inner()),
            rule => unreachable!("Expr::parse expected atom, found {:?}", rule),
        })
        .map_infix(|lhs, op, rhs| {
            let bin_op = match op.as_rule() {
                Rule::add => BinOp::Add,
                Rule::sub => BinOp::Sub,
                Rule::mul => BinOp::Mul,
                Rule::div => BinOp::Div,
                Rule::pow => BinOp::Pow,
                rule => unreachable!("Expr::parse expected infix operation, found {:?}", rule),
            };
            Expr::BinaryOp {
                op: bin_op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            }
        })
        .map_prefix(|op, rhs| {
            let un_op = match op.as_rule() {
                Rule::neg => UnOp::Neg,
                rule => unreachable!("Expr::parse expected prefix operation, found {:?}", rule),
            };
            Expr::UnaryOp {
                op: un_op,
                operand: Box::new(rhs),
            }
        })
        .parse(pairs)
}

fn eval_expr(
    expr: &Expr,
    cell_ref_resolver: &impl Fn(&str) -> Option<f64>,
) -> Result<f64, FormulaError> {
    match expr {
        Expr::Number(n) => Ok(*n),
        Expr::CellRef(cr) => match cell_ref_resolver(cr) {
            Some(value) => Ok(value),
            None => Ok(0.0),
        },
        // Expr::Range(_c1, _c2) => unimplemented!(),
        Expr::BinaryOp { op, lhs, rhs } => {
            let lval = self::eval_expr(lhs, cell_ref_resolver)?;
            let rval = self::eval_expr(rhs, cell_ref_resolver)?;
            eval_binary_op(op, lval, rval)
        }
        Expr::UnaryOp { op, operand } => {
            let val = eval_expr(operand, cell_ref_resolver)?;
            Ok(eval_unary_op(op, val))
        }
        Expr::Function { name, args } => {
            let args = args
                .iter()
                .map(|arg| eval_expr(arg, cell_ref_resolver))
                .collect::<Result<Vec<f64>, FormulaError>>()?;
            let func = FUNCTION_REGISTRY
                .get(name.to_uppercase().as_str())
                .ok_or(FormulaError::UnknownFunction)?;

            func(&args)
        }
    }
}

fn eval_binary_op(op: &BinOp, lhs: f64, rhs: f64) -> Result<f64, FormulaError> {
    match op {
        BinOp::Add => Ok(lhs + rhs),
        BinOp::Sub => Ok(lhs - rhs),
        BinOp::Mul => Ok(lhs * rhs),
        BinOp::Div => {
            if rhs == 0.0 {
                return Err(FormulaError::DivBy0);
            }
            Ok(lhs / rhs)
        }
        BinOp::Pow => Ok(lhs.powf(rhs)),
    }
}

fn eval_unary_op(op: &UnOp, operand: f64) -> f64 {
    match op {
        UnOp::Neg => -operand,
    }
}

pub fn calculate(
    input: &str,
    cell_ref_resolver: &impl Fn(&str) -> Option<f64>,
) -> Result<f64, FormulaError> {
    match CellFormulaParser::parse(Rule::formula, input) {
        Ok(mut pairs) => {
            let expr = parse_expr(pairs.next().unwrap().into_inner());
            eval_expr(&expr, cell_ref_resolver)
        }
        Err(_) => Err(FormulaError::ParsingError),
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::parser::*;
    use pest::Parser;

    #[test]
    fn test_parsing() {
        assert!(CellFormulaParser::parse(Rule::formula, "= 3").is_ok());
        assert!(CellFormulaParser::parse(Rule::formula, "= 3 + 12").is_ok());
        assert!(CellFormulaParser::parse(Rule::formula, "= 3+-12").is_ok());
        assert!(CellFormulaParser::parse(Rule::formula, "= 3 + -12 / 9").is_ok());
        assert!(CellFormulaParser::parse(Rule::formula, "= a1 + -12 / 9").is_ok());
        assert!(CellFormulaParser::parse(Rule::formula, "= a1 + -B2 / 9").is_ok());
        assert!(CellFormulaParser::parse(Rule::formula, "= SUM(1,a1,-3)").is_ok());
        assert!(CellFormulaParser::parse(Rule::formula, "= sum()").is_ok());
        assert!(CellFormulaParser::parse(Rule::formula, "= a1:b3").is_err());
    }

    fn mock_cell_ref_resolver(cell_ref: &str) -> Option<f64> {
        match cell_ref {
            "a1" | "A1" => Some(1.0),
            "b2" | "B2" => Some(2.0),
            _ => None,
        }
    }

    #[test]
    fn calculate_basic_math() {
        assert_eq!(calculate("= 3 + 12", &mock_cell_ref_resolver), Ok(15.0));
        assert_eq!(calculate("= 3 + -12", &mock_cell_ref_resolver), Ok(-9.0));
        assert_eq!(
            calculate("= 3 + -12 / 3", &mock_cell_ref_resolver),
            Ok(-1.0)
        );
        assert_eq!(
            calculate("= (3 + -12) / 3", &mock_cell_ref_resolver),
            Ok(-3.0)
        );
        assert_eq!(
            calculate("= -a1 + B2 * 2", &mock_cell_ref_resolver),
            Ok(3.0)
        );
    }

    #[test]
    fn calculate_functions() {
        assert_eq!(calculate("=Sum(1,2,3)", &mock_cell_ref_resolver), Ok(6.0));
        assert_eq!(calculate("=avG(1,2,3)", &mock_cell_ref_resolver), Ok(2.0));
        assert_eq!(
            calculate("=avG()", &mock_cell_ref_resolver),
            Err(FormulaError::DivBy0)
        );
        assert_eq!(
            calculate("=Sum(a1, b2 * 3)", &mock_cell_ref_resolver),
            Ok(7.0)
        );
        assert_eq!(calculate("=avG(a1,b2,3)", &mock_cell_ref_resolver), Ok(2.0));
        assert_eq!(calculate("=sum()", &mock_cell_ref_resolver), Ok(0.0));
    }
}
