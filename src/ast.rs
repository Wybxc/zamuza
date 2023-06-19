use std::fmt::Display;

#[derive(Debug, Clone, PartialEq)]
pub struct Name(pub String);

impl Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Agent {
    pub name: String,
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

#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    Name(Name),
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

#[derive(Debug, Clone, PartialEq)]
pub struct Equation {
    pub left: Term,
    pub right: Term,
}

impl Display for Equation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} = {}", self.left, self.right)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuleTerm {
    pub agent: String,
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

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub terms: [RuleTerm; 2],
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

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub rules: Vec<Rule>,
    pub equations: Vec<Equation>,
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
