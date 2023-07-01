//! 运行时构建器。

use anyhow::{bail, Result};

use crate::ast;

use super::{AgentId, AgentMeta, Function, Initializer, Instruction, Local, Program, Rule};

struct Name(pub String);

enum ArgSlot {
    Left(usize),
    Right(usize),
}

/// 用于构建运行时的构建器。
#[derive(Default)]
pub struct RuntimeBuilder {
    global: GlobalBuilder,
    rules: RulesBuilder,
    functions: FunctionsBuilder,
}

impl RuntimeBuilder {
    /// 创建一个新的 `RuntimeBuilder`。
    pub fn new() -> Self {
        Default::default()
    }

    /// 向运行时添加一个 `Program`。
    pub fn module(&mut self, module: ast::Module) -> Result<&mut Self> {
        for rule in module.rules {
            self.rules.rule(&mut self.global, rule.into_inner())?;
        }
        for net in module.nets {
            self.functions
                .function(&mut self.global, net.into_inner())?;
        }

        Ok(self)
    }

    /// 构建运行时。
    pub fn build(self) -> Result<Program> {
        let agent_metas = self.global.build();
        let (rules, rule_map) = self.rules.build();
        let functions = self.functions.build();

        Ok(Program {
            agents: agent_metas,
            rules,
            rule_map,
            functions,
        })
    }

    /// 从 `Program` 构建运行时。
    pub fn build_runtime(program: ast::Module) -> Result<Program> {
        let mut builder = Self::new();
        builder.module(program)?;
        builder.build()
    }
}

struct GlobalBuilder {
    agents: Vec<AgentMeta>,
}

impl Default for GlobalBuilder {
    fn default() -> Self {
        Self {
            agents: vec![AgentMeta::new("$", 1)],
        }
    }
}

impl GlobalBuilder {
    pub fn add_or_get_agent(&mut self, name: &str, arity: usize) -> Result<AgentId> {
        match self
            .agents
            .iter()
            .enumerate()
            .find_map(|(id, AgentMeta { name: n, arity })| Some((id, arity)).filter(|_| n == name))
        {
            Some((id, a)) if *a == arity => Ok(AgentId(id)),
            Some((_, a)) => {
                bail!("agent `{}` has arity {}, but {} is given", name, a, arity)
            }
            None => {
                let id = self.agents.len();
                self.agents.push(AgentMeta::new(name, arity));
                Ok(AgentId(id))
            }
        }
    }

    pub fn build(self) -> Vec<AgentMeta> {
        self.agents
    }
}

#[derive(Default)]
struct BodyBuilder {
    arguments: Vec<(Name, ArgSlot)>,
    names: Vec<Name>,
    terms: Vec<AgentId>,
    instructions: Vec<Instruction>,
}

impl BodyBuilder {
    pub fn slot(&mut self, name: String, slot: ArgSlot) -> &mut Self {
        self.arguments.push((Name(name), slot));
        self
    }

    fn add_or_get_name(&mut self, name: &str) -> Local {
        if let Some(id) = self
            .arguments
            .iter()
            .enumerate()
            .find_map(|(id, (Name(n), _))| Some(id).filter(|_| *n == name))
        {
            return Local::Slot(id);
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
        Local::Name(id)
    }

    fn add_term(&mut self, agent_id: AgentId) -> Local {
        let id = self.terms.len();
        self.terms.push(agent_id);
        Local::Agent(id)
    }

    pub fn term(&mut self, global: &mut GlobalBuilder, term: ast::Term) -> Result<Local> {
        use ast::*;
        match term {
            Term::Name(name) => {
                let term_name = self.add_or_get_name(name.as_name());
                Ok(term_name)
            }
            Term::Agent(agent) => {
                let Agent { name, body } = agent.into_inner();
                let agent_id = global.add_or_get_agent(&name, body.len())?;
                let term_name = self.add_term(agent_id);

                for (i, term) in body.into_iter().enumerate() {
                    let sub_name = self.term(global, term)?;
                    self.instructions.push(Instruction::SetSlot {
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
        self.instructions.push(Instruction::PushEquation {
            left: left_name,
            right: right_name,
            description,
        });
        Ok(self)
    }

    pub fn build(self) -> Result<(Vec<Initializer>, Vec<Instruction>)> {
        let arguments =
            self.arguments
                .into_iter()
                .enumerate()
                .map(|(index, (_, slot))| match slot {
                    ArgSlot::Left(slot) => Initializer::SlotFromLeft { index, slot },
                    ArgSlot::Right(slot) => Initializer::SlotFromRight { index, slot },
                });
        let names = self
            .names
            .into_iter()
            .enumerate()
            .map(|(index, _)| Initializer::Name { index });
        let terms = self
            .terms
            .into_iter()
            .enumerate()
            .map(|(index, id)| Initializer::Agent { index, id });
        let initailizers = arguments.chain(names).chain(terms).collect::<Vec<_>>();

        Ok((initailizers, self.instructions))
    }
}

#[derive(Default)]
struct RulesBuilder {
    rules: Vec<Rule>,
    rule_map: Vec<(AgentId, AgentId, usize)>,
}

impl RulesBuilder {
    pub fn rule(&mut self, global: &mut GlobalBuilder, rule: ast::Rule) -> Result<&mut Self> {
        let description = rule.to_string();
        let ast::Rule {
            term_pair,
            equations,
        } = rule;
        let ast::RuleTermPair {
            left: term1,
            right: term2,
        } = term_pair.into_inner();

        let mut body = BodyBuilder::default();
        let a1 = global.add_or_get_agent(&term1.agent, term1.body.len())?;
        let a2 = global.add_or_get_agent(&term2.agent, term2.body.len())?;

        // 保证 left 的 AGENT_ID 小于 right 的 AGENT_ID
        let (a_left, a_right) = if a1 <= a2 { (a1, a2) } else { (a2, a1) };
        let (term_left, term_right) = if a1 <= a2 {
            (term1, term2)
        } else {
            (term2, term1)
        };

        for (i, name) in term_left.into_inner().body.into_iter().enumerate() {
            body.slot(name.as_name().to_string(), ArgSlot::Left(i + 1));
        }
        for (i, name) in term_right.into_inner().body.into_iter().enumerate() {
            body.slot(name.as_name().to_string(), ArgSlot::Right(i + 1));
        }

        for equation in equations {
            body.equation(global, equation.into_inner())?;
        }

        let (initializers, instructions) = body.build()?;
        let index = self.rules.len();
        self.rules.push(Rule {
            index,
            description,
            initializers,
            instructions,
        });
        self.rule_map.push((a_left, a_right, index));

        Ok(self)
    }

    pub fn build(self) -> (Vec<Rule>, Vec<(AgentId, AgentId, usize)>) {
        (self.rules, self.rule_map)
    }
}

#[derive(Default)]
struct FunctionsBuilder {
    functions: Vec<Function>,
}

impl FunctionsBuilder {
    pub fn function(
        &mut self,
        global: &mut GlobalBuilder,
        function: ast::Net,
    ) -> Result<&mut Self> {
        let mut body = BodyBuilder::default();
        let mut outputs = vec![];

        for equation in function.equations {
            body.equation(global, equation.into_inner())?;
        }
        for interface in function.interfaces {
            let term = body.term(global, interface)?;
            outputs.push(term);
        }

        let (initializers, instructions) = body.build()?;
        self.functions.push(Function {
            name: function.name.as_ref().to_string(),
            initializers,
            instructions,
            outputs,
        });
        Ok(self)
    }

    pub fn build(self) -> Vec<Function> {
        self.functions
    }
}
