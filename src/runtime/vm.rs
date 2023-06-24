//! 解释运行

use colorized::{Color, Colors};
use std::{cell::RefCell, collections::HashMap, rc::Rc, time::Instant};
use thiserror::Error;

use crate::options::Options;

use super::ir;

/// 运行时错误。
#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("no rule for {left} and {right}")]
    RuleNotFound { left: String, right: String },

    #[error("in function main: {0}")]
    MainError(#[source] ExecutionError),

    #[error("in rule {0}: {1}")]
    RuleError(String, #[source] ExecutionError),
}

#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("invalid instruction: {0}")]
    InvalidInstruction(String),

    #[error("slot {slot} of agent {agent} is not found when executing instruction {inst}")]
    SlotNotFound {
        agent: String,
        slot: usize,
        inst: String,
    },

    #[error("local variable {local} is uninitialized or moved when executing instruction {inst}")]
    UninitializedLocal { local: String, inst: String },

    #[error("read slot of non-agent variable {var} when executing instruction {inst}")]
    InvalidRead { var: String, inst: String },
}

/// 虚拟机。
pub struct VM {
    /// Runtime.
    runtime: Runtime,
    /// Rule map.
    rule_map: HashMap<(ir::AgentId, ir::AgentId), ir::Rule>,
    /// Main function.
    main: ir::Main,
    /// Enable tracing.
    trace: bool,
    /// Print timing information.
    timing: bool,
}

struct Runtime {
    /// The agents.
    agents: Vec<ir::AgentMeta>,
    /// Name counter.
    name_counter: usize,
    /// Equation stack.
    equation_stack: Vec<(NonNullValue, NonNullValue)>,
    /// Max stack size.
    max_stack_size: usize,
    overflowed: bool,
}

impl VM {
    /// 从 IR 构建虚拟机。
    pub fn new(program: ir::Program, options: &Options) -> Self {
        let agents = program.agents;

        let mut rules = program
            .rules
            .into_iter()
            .map(Option::Some)
            .collect::<Vec<_>>();
        let rule_map = program
            .rule_map
            .into_iter()
            .map(|(lhs, rhs, i)| ((lhs, rhs), rules[i].take().unwrap()))
            .collect();

        let main = program.main;

        Self {
            runtime: Runtime {
                agents,
                name_counter: 0,
                equation_stack: Vec::new(),
                max_stack_size: options.stack_size,
                overflowed: false,
            },
            rule_map,
            main,
            trace: options.trace,
            timing: options.timing,
        }
    }
}

#[derive(Default)]
struct StackFrame {
    names: Vec<Value>,
    agents: Vec<Value>,
    slots: Vec<Value>,
}

impl StackFrame {
    pub fn new() -> Self {
        Default::default()
    }
}

struct Term {
    id: ir::AgentId,
    slots: Vec<Value>,
}

enum Ref {
    Name(usize),
    Value(NonNullValue),
}

enum NonNullValue {
    Term(Term),
    Ref(Rc<RefCell<Ref>>),
}

enum Value {
    Term(Option<Term>),
    Ref(Rc<RefCell<Ref>>),
}

impl Term {
    pub fn slot(&mut self, index: usize) -> Option<Value> {
        Some(self.slots.get_mut(index - 1)?.take())
    }

    pub fn slot_mut(&mut self, index: usize) -> Option<&mut Value> {
        self.slots.get_mut(index - 1)
    }
}

impl From<NonNullValue> for Value {
    fn from(value: NonNullValue) -> Self {
        match value {
            NonNullValue::Term(t) => Self::Term(Some(t)),
            NonNullValue::Ref(r) => Self::Ref(r),
        }
    }
}

impl TryFrom<Value> for NonNullValue {
    type Error = ();

    fn try_from(value: Value) -> std::result::Result<Self, Self::Error> {
        match value {
            Value::Term(Some(t)) => Ok(NonNullValue::Term(t)),
            Value::Term(None) => Err(()),
            Value::Ref(r) => Ok(NonNullValue::Ref(r)),
        }
    }
}

impl Value {
    pub fn new_agent(id: ir::AgentId, slots: Vec<Value>) -> Value {
        Self::Term(Some(Term { id, slots }))
    }

    pub fn new_name(id: usize) -> Value {
        Self::Ref(Rc::new(RefCell::new(Ref::Name(id))))
    }

    pub fn take(&mut self) -> Value {
        match self {
            Value::Term(t) => Value::Term(t.take()),
            Value::Ref(r) => Value::Ref(r.clone()),
        }
    }
}

impl Runtime {
    pub fn new_agent(&self, agent_id: ir::AgentId) -> Value {
        let arity = self.agents[agent_id.0].arity;
        let mut slots = Vec::with_capacity(arity);
        slots.resize_with(arity, || Value::Term(None));
        Value::new_agent(agent_id, slots)
    }

    pub fn new_name(&mut self) -> Value {
        self.name_counter += 1;
        Value::new_name(self.name_counter)
    }

    pub fn push_equation(&mut self, lhs: NonNullValue, rhs: NonNullValue) {
        self.equation_stack.push((lhs, rhs));
        if !self.overflowed && self.equation_stack.len() > self.max_stack_size {
            self.overflowed = true;
            eprintln!("{}: stack overflow", "warning".color(Colors::YellowFg));
        }
    }

    pub fn pop_equation(&mut self) -> Option<(NonNullValue, NonNullValue)> {
        self.equation_stack.pop()
    }

    fn print_term(&self, term: &Term, max_recursion: usize) -> String {
        let Term { id, slots } = term;
        let mut s = self.agents[id.0].name.clone();
        if !slots.is_empty() {
            s.push('(');
            let mut first = true;
            for slot in slots {
                if first {
                    first = false;
                } else {
                    s.push_str(", ");
                }
                s.push_str(&self.print(slot, max_recursion - 1));
            }
            s.push(')');
        }
        s
    }

    pub fn print_non_null(&self, value: &NonNullValue, max_recursion: usize) -> String {
        if max_recursion == 0 {
            return "...".to_string();
        }
        match value {
            NonNullValue::Term(term) => self.print_term(term, max_recursion),
            NonNullValue::Ref(r) => match &*r.borrow() {
                Ref::Name(n) => format!("x{n}"),
                Ref::Value(t) => self.print_non_null(t, max_recursion),
            },
        }
    }

    pub fn print(&self, value: &Value, max_recursion: usize) -> String {
        if max_recursion == 0 {
            return "...".to_string();
        }
        match value {
            Value::Term(Some(term)) => self.print_term(term, max_recursion),
            Value::Term(None) => "?".to_string(),
            Value::Ref(r) => match &*r.borrow() {
                Ref::Name(n) => format!("x{n}"),
                Ref::Value(t) => self.print_non_null(t, max_recursion),
            },
        }
    }
}

impl VM {
    /// 运行虚拟机。
    pub fn run(mut self) -> Result<(), RuntimeError> {
        let start_time = Instant::now();
        let mut reductions = 0;

        let mut main_frame = StackFrame::new();

        main_frame
            .execute_main(
                &mut self.runtime,
                &self.main.initializers,
                &self.main.instructions,
            )
            .map_err(RuntimeError::MainError)?;

        while let Some((lhs, rhs)) = self.runtime.pop_equation() {
            reductions += 1;

            if self.trace {
                let trace = format!(
                    "{} = {}",
                    self.runtime.print_non_null(&lhs, 3),
                    self.runtime.print_non_null(&rhs, 3)
                )
                .color(Colors::BrightBlackFg);
                eprintln!("{trace}");
            }

            match lhs {
                NonNullValue::Ref(r) => match &mut *r.borrow_mut() {
                    name @ Ref::Name(_) => self.reduce_variable(name, rhs),
                    term @ Ref::Value(_) => self.reduce_indirection(term, rhs),
                },
                NonNullValue::Term(tl) => match rhs {
                    NonNullValue::Ref(r) => {
                        let lhs = NonNullValue::Term(tl);
                        match &mut *r.borrow_mut() {
                            name @ Ref::Name(_) => self.reduce_variable(name, lhs),
                            term @ Ref::Value(_) => self.reduce_indirection(term, lhs),
                        }
                    }
                    NonNullValue::Term(tr) => self.reduce_interaction(tl, tr)?,
                },
            }
        }

        for output in self.main.outputs {
            let output = main_frame.get(&output);
            println!("{}", self.runtime.print(&output, 1000));
        }

        if self.timing {
            let time = (Instant::now() - start_time).as_secs_f64();
            let reductions_per_second = f64::from(reductions) / time;
            eprintln!(
                "\n[Reductions: {reductions}, CPU time: {time}, R/s: {reductions_per_second}]"
            );
        }
        Ok(())
    }

    fn reduce_variable(&mut self, lhs: &mut Ref, rhs: NonNullValue) {
        *lhs = Ref::Value(rhs);
    }

    fn reduce_indirection(&mut self, lhs: &mut Ref, rhs: NonNullValue) {
        let mut value = Ref::Name(0);
        std::mem::swap(&mut value, lhs);
        let value = match value {
            Ref::Name(_) => unreachable!(),
            Ref::Value(v) => v,
        };

        self.runtime.push_equation(value, rhs);
    }

    fn reduce_interaction(&mut self, mut lhs: Term, mut rhs: Term) -> Result<(), RuntimeError> {
        let mut id_left = lhs.id;
        let mut id_right = rhs.id;

        if id_left > id_right {
            std::mem::swap(&mut id_left, &mut id_right);
            std::mem::swap(&mut lhs, &mut rhs);
        }

        let rule = self.rule_map.get(&(id_left, id_right));
        if let Some(rule) = rule {
            let mut frame = StackFrame::new();
            frame
                .execute(
                    &mut self.runtime,
                    &rule.initializers,
                    &rule.instructions,
                    lhs,
                    rhs,
                )
                .map_err(|e| RuntimeError::RuleError(rule.description.clone(), e))?;
        } else {
            return Err(RuntimeError::RuleNotFound {
                left: self.runtime.print_term(&lhs, 3),
                right: self.runtime.print_term(&rhs, 3),
            });
        }
        Ok(())
    }
}

impl StackFrame {
    fn prepare_index(vec: &mut Vec<Value>, index: usize) {
        if vec.len() <= index {
            vec.resize_with(index + 1, || Value::Term(None));
        }
    }

    fn get(&mut self, local: &ir::Local) -> Value {
        match local {
            ir::Local::Name(index) => self.names[*index].take(),
            ir::Local::Agent(index) => self.agents[*index].take(),
            ir::Local::Slot(index) => self.slots[*index].take(),
        }
    }

    fn get_mut(&mut self, local: &ir::Local) -> &mut Value {
        match local {
            ir::Local::Name(index) => &mut self.names[*index],
            ir::Local::Agent(index) => &mut self.agents[*index],
            ir::Local::Slot(index) => &mut self.slots[*index],
        }
    }

    fn execute_main_initializers(
        &mut self,
        rt: &mut Runtime,
        initializers: &[ir::Initializer],
    ) -> Result<(), ExecutionError> {
        for initializer in initializers {
            match initializer {
                ir::Initializer::Name { index } => {
                    Self::prepare_index(&mut self.names, *index);
                    self.names[*index] = rt.new_name();
                }
                ir::Initializer::Agent { index, id } => {
                    Self::prepare_index(&mut self.agents, *index);
                    self.agents[*index] = rt.new_agent(*id);
                }
                _ => return Err(ExecutionError::InvalidInstruction(initializer.to_string())),
            }
        }
        Ok(())
    }

    fn execute_initializers(
        &mut self,
        rt: &mut Runtime,
        initializers: &[ir::Initializer],
        mut left: Term,
        mut right: Term,
    ) -> Result<(), ExecutionError> {
        for initializer in initializers {
            match initializer {
                ir::Initializer::Name { index } => {
                    Self::prepare_index(&mut self.names, *index);
                    self.names[*index] = rt.new_name();
                }
                ir::Initializer::Agent { index, id } => {
                    Self::prepare_index(&mut self.agents, *index);
                    self.agents[*index] = rt.new_agent(*id);
                }
                ir::Initializer::SlotFromLeft { index, slot } => {
                    Self::prepare_index(&mut self.slots, *index);
                    if let Some(slot) = left.slot(*slot) {
                        self.slots[*index] = slot;
                    } else {
                        return Err(ExecutionError::SlotNotFound {
                            agent: rt.print_term(&left, 3),
                            slot: *slot,
                            inst: initializer.to_string(),
                        });
                    }
                }
                ir::Initializer::SlotFromRight { index, slot } => {
                    Self::prepare_index(&mut self.slots, *index);
                    if let Some(slot) = right.slot(*slot) {
                        self.slots[*index] = slot;
                    } else {
                        return Err(ExecutionError::SlotNotFound {
                            agent: rt.print_term(&right, 3),
                            slot: *slot,
                            inst: initializer.to_string(),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    fn execute_instructions(
        &mut self,
        rt: &mut Runtime,
        instructions: &[ir::Instruction],
    ) -> Result<(), ExecutionError> {
        for instruction in instructions {
            match instruction {
                ir::Instruction::SetSlot {
                    target,
                    slot,
                    value,
                } => {
                    let value = self.get(value);
                    let tgt = self.get_mut(target);
                    match tgt {
                        Value::Term(Some(term)) => {
                            if let Some(slot) = term.slot_mut(*slot) {
                                *slot = value;
                            } else {
                                return Err(ExecutionError::SlotNotFound {
                                    agent: rt.print_term(term, 3),
                                    slot: *slot,
                                    inst: instruction.to_string(),
                                });
                            }
                        }
                        Value::Term(None) => {
                            return Err(ExecutionError::UninitializedLocal {
                                local: target.to_string(),
                                inst: instruction.to_string(),
                            })
                        }
                        Value::Ref(_) => {
                            return Err(ExecutionError::InvalidRead {
                                var: target.to_string(),
                                inst: instruction.to_string(),
                            })
                        }
                    }
                }
                ir::Instruction::PushEquation { left, right, .. } => {
                    let left = self.get(left).try_into().map_err(|_| {
                        ExecutionError::UninitializedLocal {
                            local: left.to_string(),
                            inst: instruction.to_string(),
                        }
                    })?;
                    let right = self.get(right).try_into().map_err(|_| {
                        ExecutionError::UninitializedLocal {
                            local: right.to_string(),
                            inst: instruction.to_string(),
                        }
                    })?;
                    rt.push_equation(left, right);
                }
            }
        }
        Ok(())
    }

    pub fn execute_main(
        &mut self,
        rt: &mut Runtime,
        initializers: &[ir::Initializer],
        instructions: &[ir::Instruction],
    ) -> Result<(), ExecutionError> {
        self.execute_main_initializers(rt, initializers)?;
        self.execute_instructions(rt, instructions)?;
        Ok(())
    }

    pub fn execute(
        &mut self,
        rt: &mut Runtime,
        initializers: &[ir::Initializer],
        instructions: &[ir::Instruction],
        lhs: Term,
        rhs: Term,
    ) -> Result<(), ExecutionError> {
        self.execute_initializers(rt, initializers, lhs, rhs)?;
        self.execute_instructions(rt, instructions)?;
        Ok(())
    }
}
