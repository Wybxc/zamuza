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

use std::{fmt::Display, ops::Deref};

/// 位置信息片段
#[derive(Debug, Clone, PartialEq)]
pub struct Span<'a, T>
where
    T: 'a,
{
    inner: T,
    span: pest::Span<'a>,
}

impl<'a, T: Display> Display for Span<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<'a, T> Deref for Span<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, T> AsRef<T> for Span<'a, T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl<'a, T> Span<'a, T> {
    /// 创建一个新的 `Span`。
    pub fn new(inner: T, span: pest::Span<'a>) -> Self {
        Self { inner, span }
    }

    /// 获取位置信息。
    pub fn span(&self) -> pest::Span<'a> {
        self.span
    }

    /// 转换为内部类型。
    pub fn into_inner(self) -> T {
        self.inner
    }
}

/// 变量名称
#[derive(Debug, Clone, PartialEq)]
pub enum Name<'a> {
    /// 输入变量
    In(Span<'a, &'a str>),
    /// 输出变量
    Out(Span<'a, &'a str>),
}

impl<'a> Display for Name<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Name::In(name) => write!(f, "#{}", name),
            Name::Out(name) => write!(f, "@{}", name),
        }
    }
}

impl<'a> Name<'a> {
    /// 获取变量名称
    pub fn as_name(&self) -> &str {
        match self {
            Name::In(name) => name.as_ref(),
            Name::Out(name) => name.as_ref(),
        }
    }
}

/// 程序中的交互器
#[derive(Debug, Clone, PartialEq)]
pub struct Agent<'a> {
    /// 交互器名称
    pub name: Span<'a, &'a str>,
    /// 交互器体
    pub body: Vec<Term<'a>>,
}

impl<'a> Display for Agent<'a> {
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
pub enum Term<'a> {
    /// 名称
    Name(Span<'a, Name<'a>>),
    /// 交互器
    Agent(Span<'a, Agent<'a>>),
}

impl<'a> Display for Term<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Term::Name(name) => write!(f, "{}", name),
            Term::Agent(agent) => write!(f, "{}", agent),
        }
    }
}

/// 程序中的方程
#[derive(Debug, Clone, PartialEq)]
pub struct Equation<'a> {
    /// 方程左侧
    pub left: Term<'a>,
    /// 方程右侧
    pub right: Term<'a>,
}

impl<'a> Display for Equation<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} -> {}", self.left, self.right)
    }
}

/// 规则中的项
#[derive(Debug, Clone, PartialEq)]
pub struct RuleTerm<'a> {
    /// 交互器名称
    pub agent: Span<'a, &'a str>,
    /// 交互器体
    pub body: Vec<Span<'a, Name<'a>>>,
}

impl<'a> Display for RuleTerm<'a> {
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

/// 规则项对
#[derive(Debug, Clone, PartialEq)]
pub struct RuleTermPair<'a> {
    /// 规则项对左侧
    pub left: Span<'a, RuleTerm<'a>>,
    /// 规则项对右侧
    pub right: Span<'a, RuleTerm<'a>>,
}

impl<'a> Display for RuleTermPair<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} >> {}", self.left, self.right)
    }
}

/// 程序中的规则
#[derive(Debug, Clone, PartialEq)]
pub struct Rule<'a> {
    /// 规则中的两个项
    pub term_pair: Span<'a, RuleTermPair<'a>>,
    /// 规则中的方程
    pub equations: Vec<Span<'a, Equation<'a>>>,
}

impl<'a> Display for Rule<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} => {}",
            self.term_pair,
            if !self.equations.is_empty() {
                self.equations
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                "_".to_string()
            }
        )
    }
}

/// 程序中的网络
#[derive(Debug, Clone, PartialEq)]
pub struct Net<'a> {
    /// 网络名称
    pub name: Span<'a, &'a str>,
    /// 网络接口
    pub interfaces: Vec<Term<'a>>,
    /// 网络方程
    pub equations: Vec<Span<'a, Equation<'a>>>,
}

impl<'a> Display for Net<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} <| {} |> {}",
            self.name,
            self.interfaces
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            if !self.equations.is_empty() {
                self.equations
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                "_".to_string()
            }
        )
    }
}

/// 整个程序
#[derive(Debug, Clone, PartialEq)]
pub struct Module<'a> {
    /// 程序文件名
    pub filename: &'a str,
    /// 源代码
    pub source: &'a str,
    /// 程序中的规则
    pub rules: Vec<Span<'a, Rule<'a>>>,
    /// 程序中的网络
    pub nets: Vec<Span<'a, Net<'a>>>,
}

impl<'a> Display for Module<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "/* {} */", self.filename)?;
        for rule in &self.rules {
            writeln!(f, "{}", rule)?;
        }
        for net in &self.nets {
            writeln!(f, "{}", net)?;
        }
        Ok(())
    }
}
