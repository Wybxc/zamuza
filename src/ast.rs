//! 本模块定义了语言的语法树。
//!
//! 语法树包括以下结构体：
//! - Name：名称，用于表示变量名、交互器名等
//! - Agent：交互器，由名称和交互器体组成
//! - Term：项，包括名称和交互器
//! - Equation：方程，由左右两个项组成
//! - RuleTerm：规则中的项，由交互器名称和交互器体组成
//! - Rule：规则，由两个规则项和若干方程组成
//! - Program：整个程序，由规则、方程和接口组成
//!
//! # Examples
//!
//! ```
//! use zamuza::ast::*;
//! 
//! let name = Name("x".to_string());
//! let agent = Agent {
//!     name: "F".to_string(),
//!     body: vec![Term::Name(Name("x".to_string()))],
//! };
//! let term = Term::Agent(agent.clone());
//! let equation = Equation {
//!     left: term.clone(),
//!     right: term.clone(),
//! };
//! let rule_term = RuleTerm {
//!     agent: "G".to_string(),
//!     body: vec![Name("x".to_string())],
//! };
//! let rule = Rule {
//!     terms: [rule_term.clone(), rule_term.clone()],
//!     equations: vec![equation.clone()],
//! };
//! let program = Program {
//!     rules: vec![rule.clone()],
//!     equations: vec![equation.clone()],
//!     interface: term.clone(),
//! };
//!
//! assert_eq!(name.to_string(), "#x");
//! assert_eq!(agent.to_string(), "F(#x)");
//! assert_eq!(term.to_string(), "F(#x)");
//! assert_eq!(equation.to_string(), "F(#x) = F(#x)");
//! assert_eq!(rule_term.to_string(), "G(#x)");
//! assert_eq!(rule.to_string(), "G(#x) :-: G(#x) => F(#x) = F(#x)");
//! assert_eq!(program.to_string(), "G(#x) :-: G(#x) => F(#x) = F(#x)\nF(#x) = F(#x)\n$ = F(#x)\n");
//! ```

use std::fmt::Display;

/// 交互器名称
#[derive(Debug, Clone, PartialEq)]
pub struct Name(pub String);

impl Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// 程序中的交互器
#[derive(Debug, Clone, PartialEq)]
pub struct Agent {
    /// 交互器名称
    pub name: String,
    /// 交互器体
    pub body: Vec<Term>,
}

impl Display for Agent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.body.is_empty() {
            write!(f, "{}", self.name)
        } else {
            write!(
                f,
                "{}({})",
                self.name,
                self.body
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

/// 程序中的项
#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    /// 名称
    Name(Name),
    /// 交互器
    Agent(Agent),
}

impl Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Term::Name(name) => write!(f, "{}", name),
            Term::Agent(agent) => write!(f, "{}", agent),
        }
    }
}

/// 程序中的方程
#[derive(Debug, Clone, PartialEq)]
pub struct Equation {
    /// 方程左侧
    pub left: Term,
    /// 方程右侧
    pub right: Term,
}

impl Display for Equation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} = {}", self.left, self.right)
    }
}

/// 规则中的项
#[derive(Debug, Clone, PartialEq)]
pub struct RuleTerm {
    /// 交互器名称
    pub agent: String,
    /// 交互器体
    pub body: Vec<Name>,
}

impl Display for RuleTerm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.body.is_empty() {
            write!(f, "{}", self.agent)
        } else {
            write!(
                f,
                "{}({})",
                self.agent,
                self.body
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

/// 程序中的规则
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    /// 规则中的两个项
    pub terms: [RuleTerm; 2],
    /// 规则中的方程
    pub equations: Vec<Equation>,
}

impl Display for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} :-: {}", self.terms[0], self.terms[1])?;
        if !self.equations.is_empty() {
            write!(
                f,
                " => {}",
                self.equations
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;
        }
        Ok(())
    }
}

/// 整个程序
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// 程序中的规则
    pub rules: Vec<Rule>,
    /// 程序中的方程
    pub equations: Vec<Equation>,
    /// 程序的接口
    pub interface: Term,
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for rule in &self.rules {
            writeln!(f, "{}", rule)?;
        }
        for equation in &self.equations {
            writeln!(f, "{}", equation)?;
        }
        writeln!(f, "$ = {}", self.interface)
    }
}
