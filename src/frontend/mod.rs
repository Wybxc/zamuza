//! 编译器前端

pub mod ast;
pub mod check;
pub mod parser;
pub mod semantic;

/// 变量名称
#[derive(Debug, Clone, PartialEq)]
pub struct Name(pub String);

/// 程序中的交互器
#[derive(Debug, Clone, PartialEq)]
pub struct Agent {
    /// 交互器名称
    pub name: Name,
    /// 交互器体
    pub body: Vec<Term>,
}

/// 程序中的项
#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    /// 变量
    Var(Name),
    /// 交互器
    Agent(Agent),
}

/// 方程
#[derive(Debug, Clone, PartialEq)]
pub struct Equation {
    /// 方程左边
    pub left: Term,
    /// 方程右边
    pub right: Term,
}

/// 规则中的项
#[derive(Debug, Clone, PartialEq)]
pub struct RuleTerm {
    /// 交互器名称
    pub agent: Name,
    /// 交互器体
    pub body: Vec<Name>,
}

/// 规则
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    /// 规则左边
    pub left: RuleTerm,
    /// 规则右边
    pub right: RuleTerm,
    /// 方程
    pub equations: Vec<Equation>,
}

/// 网络
#[derive(Debug, Clone, PartialEq)]
pub struct Net {
    /// 名称
    pub name: Name,
    /// 接口
    pub interfaces: Vec<Name>,
    /// 方程
    pub equations: Vec<Equation>,
}

/// 模块
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// 规则
    pub rules: Vec<Rule>,
    /// 网络
    pub nets: Vec<Net>,
}
