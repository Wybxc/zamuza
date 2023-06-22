//! 运行时构建器。

use anyhow::{bail, Result};

use super::ir;
use crate::ast;

struct Name(pub String);

enum ArgSlot {
    Left(usize),
    Right(usize),
}

/// 用于构建运行时的构建器。
#[derive(Default)]
pub struct RuntimeBuilder {
    global: GlobalBuilder,
    interfaces: Vec<ir::Local>,
    rules: RulesBuilder,
    main: FunctionBuilder,
}

impl RuntimeBuilder {
    /// 创建一个新的 `RuntimeBuilder`。
    pub fn new() -> Self {
        Default::default()
    }

    fn term(&mut self, term: ast::Term) -> Result<ir::Local> {
        self.main.term(&mut self.global, term)
    }

    /// 向运行时添加一个 `Rule`。
    pub fn rule(&mut self, rule: ast::Rule) -> Result<&mut Self> {
        self.rules.rule(&mut self.global, rule)?;
        Ok(self)
    }

    /// 向运行时添加一个 `Equation`。
    pub fn equation(&mut self, equation: ast::Equation) -> Result<&mut Self> {
        self.main.equation(&mut self.global, equation)?;
        Ok(self)
    }

    /// 设置运行时的接口。
    pub fn interface(&mut self, interface: ast::Term) -> Result<&mut Self> {
        let term = self.term(interface)?;
        self.interfaces.push(term);
        Ok(self)
    }

    /// 向运行时添加一个 `Program`。
    pub fn program(&mut self, program: ast::Program) -> Result<&mut Self> {
        for rule in program.rules {
            self.rule(rule)?;
        }
        for equation in program.net {
            self.equation(equation)?;
        }
        for interface in program.interfaces {
            self.interface(interface)?;
        }
        Ok(self)
    }

    /// 构建运行时。
    pub fn build(self) -> Result<ir::Program> {
        let main = self.main.build()?;
        let main = ir::Main {
            initializers: main.0,
            instructions: main.1,
            outputs: self.interfaces,
        };

        let agent_metas = self.global.build();
        let (rules, rule_map) = self.rules.build();

        Ok(ir::Program {
            agents: agent_metas,
            main,
            rules,
            rule_map,
        })
    }

    /// 从 `Program` 构建运行时。
    pub fn build_runtime(program: ast::Program) -> Result<ir::Program> {
        let mut builder = Self::new();
        builder.program(program)?;
        builder.build()
    }
}

struct GlobalBuilder {
    agents: Vec<ir::AgentMeta>,
}

impl Default for GlobalBuilder {
    fn default() -> Self {
        Self {
            agents: vec![ir::AgentMeta::new("$", 1)],
        }
    }
}

impl GlobalBuilder {
    pub fn add_or_get_agent(&mut self, name: &str, arity: usize) -> Result<ir::AgentId> {
        match self
            .agents
            .iter()
            .enumerate()
            .find_map(|(id, ir::AgentMeta { name: n, arity })| {
                Some((id, arity)).filter(|_| n == name)
            }) {
            Some((id, a)) if *a == arity => Ok(ir::AgentId(id)),
            Some((_, a)) => {
                bail!("agent `{}` has arity {}, but {} is given", name, a, arity)
            }
            None => {
                let id = self.agents.len();
                self.agents.push(ir::AgentMeta::new(name, arity));
                Ok(ir::AgentId(id))
            }
        }
    }

    pub fn build(self) -> Vec<ir::AgentMeta> {
        self.agents
    }
}

#[derive(Default)]
struct FunctionBuilder {
    arguments: Vec<(Name, ArgSlot)>,
    names: Vec<Name>,
    terms: Vec<ir::AgentId>,
    body: Vec<ir::Instruction>,
}

impl FunctionBuilder {
    pub fn slot(&mut self, name: String, slot: ArgSlot) -> &mut Self {
        self.arguments.push((Name(name), slot));
        self
    }

    fn add_or_get_name(&mut self, name: &str) -> ir::Local {
        if let Some(id) = self
            .arguments
            .iter()
            .enumerate()
            .find_map(|(id, (Name(n), _))| Some(id).filter(|_| *n == name))
        {
            return ir::Local::Slot(id);
        }
        let id = match self
            .names
            .iter()
            .enumerate()
            .find_map(|(id, Name(n))| Some(id).filter(|_| *n == name))
        {
            Some(id) => id,
            None => {
                let id = self.names.len();
                self.names.push(Name(name.to_string()));
                id
            }
        };
        ir::Local::Name(id)
    }

    fn add_term(&mut self, agent_id: ir::AgentId) -> ir::Local {
        let id = self.terms.len();
        self.terms.push(agent_id);
        ir::Local::Agent(id)
    }

    pub fn term(&mut self, global: &mut GlobalBuilder, term: ast::Term) -> Result<ir::Local> {
        use ast::*;
        match term {
            Term::Name(name) => {
                let term_name = self.add_or_get_name(&name.into_name());
                Ok(term_name)
            }
            Term::Agent(Agent { name, body }) => {
                let agent_id = global.add_or_get_agent(&name, body.len())?;
                let term_name = self.add_term(agent_id);

                for (i, term) in body.into_iter().enumerate() {
                    let sub_name = self.term(global, term)?;
                    self.body.push(ir::Instruction::SetSlot {
                        target: term_name,
                        slot: i + 1,
                        value: sub_name,
                    })
                }

                Ok(term_name)
            }
        }
    }

    pub fn equation(
        &mut self,
        global: &mut GlobalBuilder,
        equation: ast::Equation,
    ) -> Result<&mut Self> {
        let description = equation.to_string();
        let ast::Equation { left, right } = equation;
        let left_name = self.term(global, left)?;
        let right_name = self.term(global, right)?;
        self.body.push(ir::Instruction::PushEquation {
            left: left_name,
            right: right_name,
            description,
        });
        Ok(self)
    }

    pub fn build(self) -> Result<(Vec<ir::Initializer>, Vec<ir::Instruction>)> {
        let arguments =
            self.arguments
                .into_iter()
                .enumerate()
                .map(|(index, (_, slot))| match slot {
                    ArgSlot::Left(slot) => ir::Initializer::SlotFromLeft { index, slot },
                    ArgSlot::Right(slot) => ir::Initializer::SlotFromRight { index, slot },
                });
        let names = self
            .names
            .into_iter()
            .enumerate()
            .map(|(index, _)| ir::Initializer::Name { index });
        let terms = self
            .terms
            .into_iter()
            .enumerate()
            .map(|(index, id)| ir::Initializer::Agent { index, id });
        let initailizers = arguments.chain(names).chain(terms).collect::<Vec<_>>();

        Ok((initailizers, self.body))
    }
}

#[derive(Default)]
struct RulesBuilder {
    rules: Vec<ir::Rule>,
    rule_map: Vec<(ir::AgentId, ir::AgentId, usize)>,
}

impl RulesBuilder {
    pub fn rule(&mut self, global: &mut GlobalBuilder, rule: ast::Rule) -> Result<&mut Self> {
        let description = rule.to_string();
        let ast::Rule {
            term_pair:
                ast::RuleTermPair {
                    left: term1,
                    right: term2,
                },
            equations,
        } = rule;

        let mut rule = FunctionBuilder::default();
        let a1 = global.add_or_get_agent(&term1.agent, term1.body.len())?;
        let a2 = global.add_or_get_agent(&term2.agent, term2.body.len())?;

        // 保证 left 的 AGENT_ID 小于 right 的 AGENT_ID
        let (a_left, a_right) = if a1 <= a2 { (a1, a2) } else { (a2, a1) };
        let (term_left, term_right) = if a1 <= a2 {
            (term1, term2)
        } else {
            (term2, term1)
        };

        for (i, name) in term_left.body.into_iter().enumerate() {
            rule.slot(name.into_name(), ArgSlot::Left(i + 1));
        }
        for (i, name) in term_right.body.into_iter().enumerate() {
            rule.slot(name.into_name(), ArgSlot::Right(i + 1));
        }

        for equation in equations {
            rule.equation(global, equation)?;
        }

        let (initializers, instructions) = rule.build()?;
        let index = self.rules.len();
        self.rules.push(ir::Rule {
            index,
            description,
            initializers,
            instructions,
        });
        self.rule_map.push((a_left, a_right, index));

        Ok(self)
    }

    pub fn build(self) -> (Vec<ir::Rule>, Vec<(ir::AgentId, ir::AgentId, usize)>) {
        (self.rules, self.rule_map)
    }
}
