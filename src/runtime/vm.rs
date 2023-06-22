//! 解释运行

use anyhow::{bail, Result};
use colorized::{Color, Colors};
use std::{cell::RefCell, collections::HashMap, rc::Rc, time::Instant};

use crate::options::Options;

use super::ir;

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
    equation_stack: Vec<(PValue, PValue)>,
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
    names: Vec<PValue>,
    agents: Vec<PValue>,
    slots: Vec<PValue>,
}

impl StackFrame {
    pub fn new() -> Self {
        Default::default()
    }
}

type PValue = Option<Rc<RefCell<Value>>>;
enum Value {
    Agent { id: ir::AgentId, slots: Vec<PValue> },
    Name { id: usize },
    Ref(PValue),
}

impl Value {
    fn new_agent(id: ir::AgentId, slots: Vec<PValue>) -> PValue {
        Some(Rc::new(RefCell::new(Self::Agent { id, slots })))
    }

    fn new_name(id: usize) -> PValue {
        Some(Rc::new(RefCell::new(Self::Name { id })))
    }

    pub fn slot(&self, index: usize) -> Option<PValue> {
        match self {
            Self::Agent { slots, .. } => slots.get(index - 1).cloned(),
            _ => None,
        }
    }

    pub fn slot_mut(&mut self, index: usize) -> Option<&mut PValue> {
        match self {
            Self::Agent { slots, .. } => slots.get_mut(index - 1),
            _ => None,
        }
    }
}

impl Runtime {
    pub fn new_agent(&self, agent_id: ir::AgentId) -> PValue {
        let arity = self.agents[agent_id.0].arity;
        let slots = vec![None; arity];
        Value::new_agent(agent_id, slots)
    }

    pub fn new_name(&mut self) -> PValue {
        self.name_counter += 1;
        Value::new_name(self.name_counter)
    }

    pub fn push_equation(&mut self, lhs: PValue, rhs: PValue) {
        self.equation_stack.push((lhs, rhs));
        if !self.overflowed && self.equation_stack.len() > self.max_stack_size {
            self.overflowed = true;
            eprintln!("{}", "warning: stack overflow".color(Colors::YellowFg));
        }
    }

    pub fn pop_equation(&mut self) -> Option<(PValue, PValue)> {
        self.equation_stack.pop()
    }

    pub fn print(&self, value: &Value, max_recursion: usize) -> Result<String> {
        if max_recursion == 0 {
            return Ok("...".to_string());
        }
        match value {
            Value::Agent { id, slots } => {
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
                        if let Some(slot) = slot {
                            s.push_str(&self.print(&slot.as_ref().borrow(), max_recursion - 1)?);
                        } else {
                            s.push('?');
                        }
                    }
                    s.push(')');
                }
                Ok(s)
            }
            Value::Name { id } => Ok(format!("x{}", id)),
            Value::Ref(value) => {
                if let Some(value) = value {
                    self.print(&value.as_ref().borrow(), max_recursion - 1)
                } else {
                    bail!("runtime error: value is None");
                }
            }
        }
    }
}

impl VM {
    /// 运行虚拟机。
    pub fn run(mut self) -> Result<()> {
        let start_time = Instant::now();
        let mut reductions = 0;

        let mut main_frame = StackFrame::new();

        main_frame
            .execute(
                &mut self.runtime,
                &self.main.initializers,
                &self.main.instructions,
                None,
                None,
            )
            .map_err(|e| anyhow::anyhow!("in function main: {:?}", e))?;

        while !self.runtime.equation_stack.is_empty() {
            let (lhs, rhs) = self.runtime.pop_equation().unwrap();
            if lhs.is_none() {
                bail!("runtime error: lhs is None");
            }
            if rhs.is_none() {
                bail!("runtime error: rhs is None");
            }
            let lhs = lhs.unwrap();
            let rhs = rhs.unwrap();

            reductions += 1;

            if self.trace {
                let trace = format!(
                    "{} = {}",
                    self.runtime.print(&lhs.as_ref().borrow(), 10)?,
                    self.runtime.print(&rhs.as_ref().borrow(), 10)?
                )
                .color(Colors::BrightBlackFg);
                eprintln!("{trace}");
            }

            // Indirection
            if let Value::Ref(value) = &*lhs.as_ref().borrow() {
                self.runtime.push_equation(value.clone(), Some(rhs));
                continue;
            };
            if let Value::Ref(value) = &*rhs.as_ref().borrow() {
                self.runtime.push_equation(Some(lhs), value.clone());
                continue;
            };

            // Interaction
            if let Value::Agent {
                id: id_left,
                slots: slots_left,
            } = &*lhs.as_ref().borrow()
            {
                if let Value::Agent {
                    id: id_right,
                    slots: slots_right,
                } = &*rhs.as_ref().borrow()
                {
                    let mut id_left = id_left;
                    let mut id_right = id_right;
                    let mut slots_left = slots_left;
                    let mut slots_right = slots_right;
                    let mut lhs = lhs.clone();
                    let mut rhs = rhs.clone();

                    if *id_left > *id_right {
                        std::mem::swap(&mut id_left, &mut id_right);
                        std::mem::swap(&mut slots_left, &mut slots_right);
                        std::mem::swap(&mut lhs, &mut rhs);
                    }

                    let rule = self.rule_map.get(&(*id_left, *id_right));
                    if let Some(rule) = rule {
                        let mut frame = StackFrame::new();
                        frame
                            .execute(
                                &mut self.runtime,
                                &rule.initializers,
                                &rule.instructions,
                                Some(lhs.clone()),
                                Some(rhs.clone()),
                            )
                            .map_err(|e| anyhow::anyhow!("in rule {}: {:?}", rule.index, e))?;
                    } else {
                        bail!(
                            "runtime error: no rule for {} and {}",
                            self.runtime.print(&lhs.as_ref().borrow(), 3)?,
                            self.runtime.print(&rhs.as_ref().borrow(), 3)?
                        );
                    }

                    continue;
                }
            };

            // Variable
            let mut left = lhs.borrow_mut();
            if let Value::Name { .. } = &*left {
                *left = Value::Ref(Some(rhs.clone()));
                continue;
            }
            drop(left);

            let mut right = rhs.borrow_mut();
            if let Value::Name { .. } = &*right {
                *right = Value::Ref(Some(lhs.clone()));
                continue;
            }
            drop(right);
        }

        let output = main_frame.get(&self.main.output);
        if let Some(output) = output {
            println!("{}", self.runtime.print(&output.as_ref().borrow(), 1000)?);
        } else {
            bail!("runtime error: output is None");
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
}

impl StackFrame {
    fn prepare_index(vec: &mut Vec<PValue>, index: usize) {
        if vec.len() <= index {
            vec.resize(index + 1, None);
        }
    }

    fn get(&self, local: &ir::Local) -> PValue {
        match local {
            ir::Local::Name(index) => self.names[*index].clone(),
            ir::Local::Agent(index) => self.agents[*index].clone(),
            ir::Local::Slot(index) => self.slots[*index].clone(),
        }
    }

    pub fn execute(
        &mut self,
        rt: &mut Runtime,
        initializers: &[ir::Initializer],
        instructions: &[ir::Instruction],
        lhs: PValue,
        rhs: PValue,
    ) -> Result<()> {
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
                    if let Some(ref left) = lhs {
                        let left = left.as_ref().borrow();
                        if let Some(slot) = left.slot(*slot) {
                            self.slots[*index] = slot;
                        } else {
                            bail!(
                                "runtime error: slot {slot} not found in {left} [{initializer}]",
                                left = rt.print(&left, 3)?
                            )
                        }
                    } else {
                        bail!("runtime error: lhs is None [{initializer}]")
                    }
                }
                ir::Initializer::SlotFromRight { index, slot } => {
                    Self::prepare_index(&mut self.slots, *index);
                    if let Some(ref right) = rhs {
                        let right = right.as_ref().borrow();
                        if let Some(slot) = right.slot(*slot) {
                            self.slots[*index] = slot;
                        } else {
                            bail!(
                                "runtime error: slot {slot} not found in {right} [{initializer}]",
                                right = rt.print(&right, 3)?
                            )
                        }
                    } else {
                        bail!("runtime error: rhs is None [{initializer}]")
                    }
                }
            }
        }

        for instruction in instructions {
            match instruction {
                ir::Instruction::SetSlot {
                    target,
                    slot,
                    value,
                } => {
                    if let Some(target) = self.get(target) {
                        let mut tgt = target.borrow_mut();
                        if let Some(slot) = tgt.slot_mut(*slot) {
                            if let Some(value) = self.get(value) {
                                *slot = Some(value);
                            } else {
                                bail!("runtime error: {value} is None [{instruction}]")
                            }
                        } else {
                            bail!(
                                "runtime error: slot {slot} not found in {target} [{instruction}]",
                                target = rt.print(&tgt, 3)?,
                            )
                        }
                    } else {
                        bail!("runtime error: {target} is None [{instruction}]")
                    }
                }
                ir::Instruction::PushEquation { left, right, .. } => {
                    if let Some(left) = self.get(left) {
                        if let Some(right) = self.get(right) {
                            rt.push_equation(Some(left), Some(right));
                        } else {
                            bail!("runtime error: {right} is None [{instruction}]")
                        }
                    } else {
                        bail!("runtime error: {left} is None [{instruction}]")
                    }
                }
            }
        }
        Ok(())
    }
}
