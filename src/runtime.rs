use anyhow::{bail, Result};

use crate::ast;

const PRELUDE: &str = r#"
#include <stdio.h>
#include <stdlib.h>

size_t* EQ_STACK[1000][2];
size_t EQ_STACK_SIZE = 0;

typedef void (*RuleFun)(size_t* left, size_t* right);

size_t* new_agent(size_t agent_id);
size_t* new_name();
void push_equation(size_t* left, size_t* right);
void pop_equation(size_t** left, size_t** right);
void print_term(size_t* term, size_t max_recursion);
void init_rules();
void run();

// define AGENT_COUNT ...     // len(AGENTS)
// chat* AGENTS[] = { ... };  // name of agents
// size_t ARITY[] = { ... };  // arity of agents
// size_t NAME_COUNTER = ...; // initial to be AGENT_COUNT
"#;

const RUNTIME: &str = r#"
RuleFun RULES[AGENT_COUNT][AGENT_COUNT] = { NULL };

#define IS_NAME(term) ((term)[0] >= AGENT_COUNT)
#define IS_AGENT(term) ((term)[0] < AGENT_COUNT)

size_t* new_agent(size_t agent_id) {
    size_t arity = ARITY[agent_id];
    size_t* agent = malloc(sizeof(size_t) * (arity + 1));
    agent[0] = agent_id;
    return agent;
}

size_t* new_name() {
    size_t* name = malloc(sizeof(size_t) * 2);
    name[0] = NAME_COUNTER++;
    name[1] = 0;
    return name;
}

void push_equation(size_t* left, size_t* right) {
    EQ_STACK[EQ_STACK_SIZE][0] = left;
    EQ_STACK[EQ_STACK_SIZE][1] = right;
    EQ_STACK_SIZE++;
}

void pop_equation(size_t** left, size_t** right) {
    EQ_STACK_SIZE--;
    *left = EQ_STACK[EQ_STACK_SIZE][0];
    *right = EQ_STACK[EQ_STACK_SIZE][1];
}

void print_term(size_t* term, size_t max_recursion) {
    if (term[0] == 0) {                 // the `$` agent
        print_term((size_t*) term[1], max_recursion);
        return;
    }
    if (IS_NAME(term)) {       // name
        printf("x%zu", term[0]);
        return;
    }

    size_t arity = ARITY[term[0]];
    if (arity == 0) {
        printf("%s", AGENTS[term[0]]);
        return;
    }

    printf("%s(", AGENTS[term[0]]);
    if (max_recursion > 0) {
        for (size_t i = 1; i <= arity; i++) {
            print_term((size_t*) term[i], max_recursion - 1);
            if (i != arity) {
                printf(", ");
            }
        }
    } else {
        printf("...");
    }
    printf(")");
}

void run() {
    size_t *left, *right;

    init_rules();

    while (EQ_STACK_SIZE) {
        pop_equation(&left, &right);

#ifdef DEBUG
        printf("equation: ");
        print_term(left, 3);
        printf(" = ");
        print_term(right, 3);
        printf("\n");
#endif

        // Indirection
        if (left[0] == 0) {
            push_equation((size_t*) left[1], right);
            free(left);
            continue;
        }
        if (right[0] == 0) {
            push_equation(left, (size_t*) right[1]);
            free(right);
            continue;
        }

        // Interaction
        if (IS_AGENT(left) && IS_AGENT(right)) {
            if (RULES[left[0]][right[0]]) {
                RULES[left[0]][right[0]](left, right);
                continue;
            }
            printf("error: no rule for ");
            print_term(left, 3);
            printf(" and ");
            print_term(right, 3);
            printf("\n");
            exit(1);
        }

        // Variable
        if (IS_NAME(left)) {
            left[0] = 0;
            left[1] = (size_t) right;
            continue;
        }
        if (IS_NAME(right)) {
            right[0] = 0;
            right[1] = (size_t) left;
            continue;
        }
    }
}
"#;

struct Arity(pub usize);
struct Name(pub String);
struct AgentId(pub usize);

#[derive(Default)]
pub struct RuntimeBuilder {
    global: GlobalBuilder,
    interface: Option<String>,
    rules: RulesBuilder,
    main: FunctionBuilder,
}

impl RuntimeBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn term(&mut self, term: ast::Term) -> Result<String> {
        self.main.term(&mut self.global, term)
    }

    pub fn rule(&mut self, rule: ast::Rule) -> Result<&mut Self> {
        self.rules.rule(&mut self.global, rule)?;
        Ok(self)
    }

    pub fn equation(&mut self, equation: ast::Equation) -> Result<&mut Self> {
        self.main.equation(&mut self.global, equation)?;
        Ok(self)
    }

    pub fn interface(&mut self, interface: ast::Term) -> Result<&mut Self> {
        self.interface = Some(self.term(interface)?);
        Ok(self)
    }

    pub fn program(&mut self, program: ast::Program) -> Result<&mut Self> {
        for rule in program.rules {
            self.rule(rule)?;
        }
        for equation in program.equations {
            self.equation(equation)?;
        }
        self.interface(program.interface)
    }

    pub fn build(mut self) -> Result<String> {
        let interface = match self.interface {
            Some(interface) => interface,
            None => bail!("interface is not given"),
        };

        self.main.signature("int main()".into()).after(format!(
            r#"
    run();
    print_term({interface}, 1000);
    return 0;
            "#
        ));

        let main = self.main.build()?;
        let global = self.global.build();
        let rules = self.rules.build()?;

        Ok(format!(
            r#"
{PRELUDE}
{global}
{RUNTIME}
{rules}
{main}
        "#
        ))
    }
}

struct GlobalBuilder {
    agents: Vec<(String, Arity)>,
}

impl Default for GlobalBuilder {
    fn default() -> Self {
        Self {
            agents: vec![("$".into(), Arity(1))],
        }
    }
}

impl GlobalBuilder {
    pub fn add_or_get_agent(&mut self, name: &str, arity: usize) -> Result<usize> {
        match self
            .agents
            .iter()
            .enumerate()
            .find_map(|(id, (n, a))| Some((id, a)).filter(|_| n == name))
        {
            Some((id, Arity(a))) if *a == arity => Ok(id),
            Some((_, Arity(a))) => {
                bail!("agent `{}` has arity {}, but {} is given", name, a, arity)
            }
            None => {
                let id = self.agents.len();
                self.agents.push((name.to_string(), Arity(arity)));
                Ok(id)
            }
        }
    }

    pub fn build(self) -> String {
        let agents_arity = self
            .agents
            .iter()
            .map(|(_, Arity(arity))| arity.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let agents_names = self
            .agents
            .iter()
            .map(|(name, _)| format!("\"{}\"", name))
            .collect::<Vec<_>>()
            .join(", ");

        let agents_count = self.agents.len();

        format!(
            r#"
#define AGENT_COUNT {agents_count}
char* AGENTS[] = {{ {agents_names} }};
size_t ARITY[] = {{ {agents_arity} }};
size_t NAME_COUNTER = AGENT_COUNT;
        "#
        )
    }
}

#[derive(Default)]
struct FunctionBuilder {
    arguments: Vec<(Name, String)>,
    names: Vec<Name>,
    terms: Vec<AgentId>,
    body: Vec<String>,
    signature: Option<String>,
    before: Option<String>,
    after: Option<String>,
}

impl FunctionBuilder {
    pub fn argument(&mut self, name: String, expr: String) -> &mut Self {
        self.arguments.push((Name(name), expr));
        self
    }

    fn add_or_get_name(&mut self, name: &str) -> String {
        if let Some(id) = self
            .arguments
            .iter()
            .enumerate()
            .find_map(|(id, (Name(n), _))| Some(id).filter(|_| *n == name))
        {
            return format!("a{}", id);
        }
        let name_id = match self
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
        format!("x{name_id}")
    }

    fn add_term(&mut self, agent_id: usize) -> String {
        let id = self.terms.len();
        self.terms.push(AgentId(agent_id));
        format!("t{id}")
    }

    pub fn signature(&mut self, signature: String) -> &mut Self {
        self.signature = Some(signature);
        self
    }

    pub fn before(&mut self, before: String) -> &mut Self {
        self.before = Some(before);
        self
    }

    pub fn after(&mut self, after: String) -> &mut Self {
        self.after = Some(after);
        self
    }

    pub fn term(&mut self, global: &mut GlobalBuilder, term: ast::Term) -> Result<String> {
        use ast::*;
        match term {
            Term::Name(Name(name)) => {
                let term_name = self.add_or_get_name(&name);
                Ok(term_name)
            }
            Term::Agent(Agent { name, body }) => {
                let agent_id = global.add_or_get_agent(&name, body.len())?;
                let term_name = self.add_term(agent_id);

                for (i, term) in body.into_iter().enumerate() {
                    let sub_name = self.term(global, term)?;
                    self.body.push(format!(
                        "{term_name}[{j}] = (size_t) {sub_name};",
                        j = i + 1
                    ));
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
        let ast::Equation { left, right } = equation;
        let left_name = self.term(global, left)?;
        let right_name = self.term(global, right)?;
        self.body.push(format!(
            "push_equation({left_name}, {right_name});",
            left_name = left_name,
            right_name = right_name
        ));
        Ok(self)
    }

    pub fn build(self) -> Result<String> {
        let signature = match self.signature {
            Some(signature) => signature,
            None => bail!("signature is not given"),
        };

        let body = self
            .body
            .into_iter()
            .map(|s| format!("    {}", s))
            .collect::<Vec<_>>()
            .join("\n");

        let before = self.before.unwrap_or_default();
        let after = self.after.unwrap_or_default();

        let arguments = self
            .arguments
            .iter()
            .enumerate()
            .map(|(id, (_, expr))| format!("    size_t* a{id} = {expr};"))
            .collect::<Vec<_>>()
            .join("\n");

        let names = self
            .names
            .iter()
            .enumerate()
            .map(|(id, _)| format!("    size_t* x{id} = new_name();"))
            .collect::<Vec<_>>()
            .join("\n");

        let terms = self
            .terms
            .iter()
            .enumerate()
            .map(|(id, AgentId(a))| format!("    size_t* t{id} = new_agent({a});"))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(format!(
            r#"
{signature} {{
{before}
{arguments}
{names}
{terms}
{body}
{after}
}}
        "#
        ))
    }
}

#[derive(Default)]
struct RulesBuilder {
    rules: Vec<(usize, usize)>,
    functions: Vec<String>,
}

impl RulesBuilder {
    pub fn rule(&mut self, global: &mut GlobalBuilder, rule: ast::Rule) -> Result<&mut Self> {
        let ast::Rule {
            terms: [term1, term2],
            equations,
        } = rule;

        let id = self.rules.len();

        let mut function = FunctionBuilder::default();
        let a1 = global.add_or_get_agent(&term1.agent, term1.body.len())?;
        let a2 = global.add_or_get_agent(&term2.agent, term2.body.len())?;
        function
            .signature(format!(
                "void rule{id} /* {n1}, {n2} */ (size_t* left, size_t* right)",
                n1 = term1.agent,
                n2 = term2.agent
            ))
            .before(format!(
                r#"
    if (left[0] != {a1}) {{
        size_t* tmp = left;
        left = right;
        right = tmp;
    }}
"#
            ));

        for (i, name) in term1.body.into_iter().enumerate() {
            function.argument(name.0, format!("(size_t*) left[{j}]", j = i + 1));
        }
        for (i, name) in term2.body.into_iter().enumerate() {
            function.argument(name.0, format!("(size_t*) right[{j}]", j = i + 1));
        }

        for equation in equations {
            function.equation(global, equation)?;
        }

        function.after(
            r#"
    free(left);
    free(right);
    "#
            .to_string(),
        );

        self.rules.push((a1, a2));
        self.functions.push(function.build()?);

        Ok(self)
    }

    pub fn build(self) -> Result<String> {
        let rules = self
            .rules
            .iter()
            .enumerate()
            .map(|(id, (a1, a2))| {
                format!("    RULES[{a1}][{a2}] = rule{id};\n    RULES[{a2}][{a1}] = rule{id};")
            })
            .collect::<Vec<_>>()
            .join("\n");

        let functions = self.functions.join("\n");

        Ok(format!(
            r#"
{functions}

void init_rules() {{
{rules}
}}
"#
        ))
    }
}
