//! 运行时构建器。

use std::fmt::Display;

pub mod builder;
pub mod optimize;
pub mod target;

pub use builder::RuntimeBuilder;

/// Agent ID
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct AgentId(pub usize);

impl Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Local variable
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Local {
    /// Name
    Name(usize),
    /// Agent
    Agent(usize),
    /// Slot
    Slot(usize),
}

impl Display for Local {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Local::Name(index) => write!(f, "x{}", index),
            Local::Agent(index) => write!(f, "a{}", index),
            Local::Slot(index) => write!(f, "s{}", index),
        }
    }
}

/// Initializer
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub enum RuleInitializer {
    /// Name
    Name { index: usize },
    /// Agent
    Agent { index: usize, id: AgentId },
    /// Slot value from left argument
    SlotFromLeft { index: usize, slot: usize },
    /// Slot value from right argument
    SlotFromRight { index: usize, slot: usize },
    /// Reuse agent from left argument
    ReuseLeft { index: usize },
    /// Reuse agent from right argument
    ReuseRight { index: usize },
}

impl Display for RuleInitializer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleInitializer::Name { index } => write!(f, "let x{} = new_name();", index),
            RuleInitializer::Agent { index, id } => {
                write!(f, "let a{} = new_agent({});", index, id)
            }
            RuleInitializer::SlotFromLeft { index, slot } => {
                write!(f, "let s{} = left[{}];", index, slot)
            }
            RuleInitializer::SlotFromRight { index, slot } => {
                write!(f, "let s{} = right[{}];", index, slot)
            }
            RuleInitializer::ReuseLeft { index } => write!(f, "let a{} = left;", index),
            RuleInitializer::ReuseRight { index } => write!(f, "let a{} = right;", index),
        }
    }
}

/// Initializer
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub enum NetInitializer {
    /// Name
    Name { index: usize },
    /// Agent
    Agent { index: usize, id: AgentId },
}

impl Display for NetInitializer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetInitializer::Name { index } => write!(f, "let x{} = new_name();", index),
            NetInitializer::Agent { index, id } => {
                write!(f, "let a{} = new_agent({});", index, id)
            }
        }
    }
}

/// Instruction
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub enum RuleInstruction {
    /// `target[slot] = value;`
    SetSlot {
        target: Local,
        slot: usize,
        value: Local,
    },
    /// `push_equation(left, right);`
    PushEquation {
        left: Local,
        right: Local,
        description: String,
    },
    /// `free(left);`
    FreeLeft,
    /// `free(right);`
    FreeRight,
}

impl Display for RuleInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleInstruction::SetSlot {
                target,
                slot,
                value,
            } => {
                write!(f, "{}[{}] = {};", target, slot, value)
            }
            RuleInstruction::PushEquation {
                left,
                right,
                description,
            } => write!(f, "push_equation({}, {}); // {}", left, right, description),
            RuleInstruction::FreeLeft => write!(f, "free(left);"),
            RuleInstruction::FreeRight => write!(f, "free(right);"),
        }
    }
}

/// Instruction
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub enum NetInstruction {
    /// `target[slot] = value;`
    SetSlot {
        target: Local,
        slot: usize,
        value: Local,
    },
    /// `push_equation(left, right);`
    PushEquation {
        left: Local,
        right: Local,
        description: String,
    },
}

impl Display for NetInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetInstruction::SetSlot {
                target,
                slot,
                value,
            } => {
                write!(f, "{}[{}] = {};", target, slot, value)
            }
            NetInstruction::PushEquation {
                left,
                right,
                description,
            } => write!(f, "push_equation({}, {}); // {}", left, right, description),
        }
    }
}

/// Agent metadata
#[derive(Clone, Debug)]
pub struct AgentMeta {
    /// Name
    pub name: String,
    /// Arity
    pub arity: usize,
}

impl Display for AgentMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "let {} = define_agent({});", self.name, self.arity)
    }
}

impl AgentMeta {
    /// Create a new agent metadata.
    pub fn new(name: impl Into<String>, arity: usize) -> Self {
        Self {
            name: name.into(),
            arity,
        }
    }
}

/// Rule
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct Rule {
    pub index: usize,
    pub description: String,
    pub initializers: Vec<RuleInitializer>,
    pub instructions: Vec<RuleInstruction>,
}

impl Display for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "// {}", self.description)?;
        writeln!(f, "function rule_{}(left, right) {{", self.index)?;
        for initializer in &self.initializers {
            writeln!(f, "    {}", initializer)?;
        }
        for instruction in &self.instructions {
            writeln!(f, "    {}", instruction)?;
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}

/// function
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct Function {
    pub index: usize,
    pub initializers: Vec<NetInitializer>,
    pub instructions: Vec<NetInstruction>,
    pub outputs: Vec<Local>,
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "export function func_{}() {{", self.index)?;
        for initializer in &self.initializers {
            writeln!(f, "    {}", initializer)?;
        }
        for instruction in &self.instructions {
            writeln!(f, "    {}", instruction)?;
        }
        writeln!(
            f,
            "    return {}",
            self.outputs
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )?;
        writeln!(f, "}}")?;
        Ok(())
    }
}

#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct FunctionMeta {
    pub name: String,
    pub output_count: usize,
}

impl Display for FunctionMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "let {} = define_function({});",
            self.name, self.output_count
        )?;
        Ok(())
    }
}

/// Program
#[derive(Clone, Debug)]
pub struct Program {
    /// Agents defined in the program
    pub agents: Vec<AgentMeta>,
    /// Rules
    pub rules: Vec<Rule>,
    /// Rule map (left, right, rule_id)
    pub rule_map: Vec<(AgentId, AgentId, usize)>,
    /// Functions
    pub functions: Vec<Function>,
    /// Function metadata
    pub function_meta: Vec<FunctionMeta>,
    /// Entry point
    pub entry_point: usize,
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "// Agents")?;
        for agent_meta in &self.agents {
            writeln!(f, "{}", agent_meta)?;
        }
        writeln!(f)?;

        writeln!(f, "// Rules")?;
        for rule in &self.rules {
            writeln!(f, "{}", rule)?;
        }

        writeln!(f, "function init_rules() {{")?;
        for (left, right, rule_id) in &self.rule_map {
            writeln!(f, "    rules[{}][{}] = rule_{};", left.0, right.0, rule_id)?;
        }
        writeln!(f, "}}")?;
        writeln!(f)?;

        writeln!(f, "// Functions")?;
        for function in &self.functions {
            writeln!(f, "{}", function)?;
        }

        writeln!(
            f,
            "{}",
            r#"
function main() {{
    outputs = f_Main();
    init_rules();
    run();
    for (output of outputs) {{
        print(output);
    }}
}}
"#
            .trim_start()
        )?;
        Ok(())
    }
}
