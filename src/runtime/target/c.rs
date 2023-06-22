//! 编译到 C 语言的运行时

use anyhow::Result;

use crate::options::Options;

use super::ir;

/// 编译到 C 语言的运行时
pub struct C;

impl super::Target for C {
    fn write(mut f: impl std::io::Write, program: ir::Program, options: &Options) -> Result<()> {
        Self::write_includes(&mut f, options)?;
        Self::write_prelude(&mut f, options)?;
        Self::write_global(&mut f, program.agents)?;
        Self::write_runtime(&mut f)?;

        for rule in program.rules {
            Self::write_rule(&mut f, rule)?;
        }

        Self::write_rule_map(&mut f, program.rule_map)?;
        Self::write_main(&mut f, program.main)?;
        Ok(())
    }
}

impl C {
    const INCLUDES: &str = r#"
#include <stdio.h>
#include <stdlib.h>
"#;

    fn write_includes(mut f: impl std::io::Write, options: &Options) -> Result<()> {
        f.write_all(C::INCLUDES.trim_start().as_bytes())?;

        if options.timing {
            writeln!(f, "#include <time.h>")?;
            writeln!(f, "#define ZZ_TIMING")?;
        }
        if options.trace {
            writeln!(f, "#define ZZ_TRACE")?;
        }

        Ok(())
    }

    const PRELUDE: &str = r#"
size_t* EQ_STACK[MAX_STACK_SIZE][2];
size_t EQ_STACK_SIZE = 0;

#ifdef ZZ_TIMING
size_t REDUCTIONS = 0;
#endif

typedef void (*RuleFun)(size_t* left, size_t* right);

size_t* new_agent(size_t agent_id);
size_t* new_name();
void push_equation(size_t* left, size_t* right);
void pop_equation(size_t** left, size_t** right);
void print_term(FILE* f, size_t* term, size_t max_recursion);
void init_rules();
void run();
"#;

    fn write_prelude(mut f: impl std::io::Write, options: &Options) -> Result<()> {
        writeln!(f, "#define MAX_STACK_SIZE {}", options.stack_size)?;
        f.write_all(C::PRELUDE.as_bytes())?;
        Ok(())
    }

    fn write_global(mut f: impl std::io::Write, agents: Vec<ir::AgentMeta>) -> Result<()> {
        let agents_count = agents.len();
        let agents_arity = agents
            .iter()
            .map(|meta| meta.arity.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let agents_names = agents
            .iter()
            .map(|meta| format!("\"{}\"", meta.name))
            .collect::<Vec<_>>()
            .join(", ");

        write!(
            f,
            r#"
#define AGENT_COUNT {agents_count}
char* AGENTS[] = {{ {agents_names} }};
size_t ARITY[] = {{ {agents_arity} }};
size_t NAME_COUNTER = AGENT_COUNT;
"#
        )?;

        Ok(())
    }

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
    if (EQ_STACK_SIZE >= MAX_STACK_SIZE) {
        fprintf(stderr, "error: stack overflow\n");
        exit(1);
    }
    EQ_STACK[EQ_STACK_SIZE][0] = left;
    EQ_STACK[EQ_STACK_SIZE][1] = right;
    EQ_STACK_SIZE++;
}

void pop_equation(size_t** left, size_t** right) {
    EQ_STACK_SIZE--;
    *left = EQ_STACK[EQ_STACK_SIZE][0];
    *right = EQ_STACK[EQ_STACK_SIZE][1];
}

void print_term(FILE* f, size_t* term, size_t max_recursion) {
    if (term[0] == 0) {        // the `$` agent
        print_term(f, (size_t*) term[1], max_recursion);
        return;
    }
    if (IS_NAME(term)) {       // name
        fprintf(f, "x%zu", term[0]);
        return;
    }

    size_t arity = ARITY[term[0]];
    if (arity == 0) {
        fprintf(f, "%s", AGENTS[term[0]]);
        return;
    }

    fprintf(f, "%s(", AGENTS[term[0]]);
    if (max_recursion > 0) {
        for (size_t i = 1; i <= arity; i++) {
            print_term(f, (size_t*) term[i], max_recursion - 1);
            if (i != arity) {
                fprintf(f, ", ");
            }
        }
    } else {
        fprintf(f, "...");
    }
    fprintf(f, ")");
}

void run() {
    size_t *left, *right;

    init_rules();

    while (EQ_STACK_SIZE) {
        pop_equation(&left, &right);
#ifdef ZZ_TIMING
        REDUCTIONS++;
#endif

#ifdef ZZ_TRACE
        fprintf(stderr, "\x1b[90m");
        print_term(stderr, left, 3);
        fprintf(stderr, " = ");
        print_term(stderr, right, 3);
        fprintf(stderr, "\x1b[0m\n");
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
            size_t a_left = left[0];
            size_t a_right = right[0];

            if (a_left <= a_right) {
                if (RULES[a_left][a_right]) {
                    RULES[a_left][a_right](left, right);
                    free(left);
                    free(right);
                    continue;
                }
            } else {
                if (RULES[a_right][a_left]) {
                    RULES[a_right][a_left](right, left);
                    free(left);
                    free(right);
                    continue;
                }
            }
            fprintf(stderr, "error: no rule for ");
            print_term(stderr, left, 3);
            fprintf(stderr, " and ");
            print_term(stderr, right, 3);
            fprintf(stderr, "\n");
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

    fn write_runtime(mut f: impl std::io::Write) -> Result<()> {
        f.write_all(C::RUNTIME.as_bytes())?;
        Ok(())
    }

    fn write_rule(mut f: impl std::io::Write, rule: ir::Rule) -> Result<()> {
        write!(
            f,
            r#"
// {description}
void rule_{index}(size_t* left, size_t* right) {{
"#,
            index = rule.index,
            description = rule.description
        )?;

        for initailizer in rule.initializers {
            Self::write_initializer(&mut f, initailizer)?;
        }
        for instruction in rule.instructions {
            Self::write_instruction(&mut f, instruction)?;
        }

        writeln!(f, "}}")?;

        Ok(())
    }

    fn write_initializer(mut f: impl std::io::Write, initializer: ir::Initializer) -> Result<()> {
        match initializer {
            ir::Initializer::Name { index } => {
                writeln!(f, "    size_t* x{index} = new_name();")?;
            }
            ir::Initializer::Agent { index, id } => {
                writeln!(f, "    size_t* a{index} = new_agent({id});")?
            }
            ir::Initializer::SlotFromLeft { index, slot } => {
                writeln!(f, "    size_t* s{index} = (size_t*) left[{slot}];",)?
            }
            ir::Initializer::SlotFromRight { index, slot } => {
                writeln!(f, "    size_t* s{index} = (size_t*) right[{slot}];",)?
            }
        }
        Ok(())
    }

    fn write_instruction(mut f: impl std::io::Write, instruction: ir::Instruction) -> Result<()> {
        match instruction {
            ir::Instruction::SetSlot {
                target,
                slot,
                value,
            } => writeln!(f, "    {target}[{slot}] = (size_t) {value};")?,
            ir::Instruction::PushEquation {
                left,
                right,
                description,
            } => writeln!(f, "    push_equation({left}, {right});  // {description}")?,
        }
        Ok(())
    }

    fn write_rule_map(
        mut f: impl std::io::Write,
        rule_map: Vec<(ir::AgentId, ir::AgentId, usize)>,
    ) -> Result<()> {
        write!(
            f,
            r#"
void init_rules() {{
"#
        )?;

        for (left, right, index) in rule_map {
            writeln!(f, "    RULES[{left}][{right}] = rule_{index};")?;
        }
        writeln!(f, "}}")?;
        Ok(())
    }

    fn write_main(mut f: impl std::io::Write, main: ir::Main) -> Result<()> {
        write!(
            f,
            r#"
int main() {{
#ifdef ZZ_TIMING
    clock_t start = clock();
#endif
"#
        )?;

        for initializer in main.initializers {
            Self::write_initializer(&mut f, initializer)?;
        }
        for instruction in main.instructions {
            Self::write_instruction(&mut f, instruction)?;
        }

        writeln!(f, "\n\n    run();")?;
        for output in main.outputs {
            writeln!(f, r#"    print_term(stdout, {output}, 1000);"#)?;
            writeln!(f, r#"    printf("\n");"#)?;
        }

        write!(
            f,
            r#"

#ifdef ZZ_TIMING
    clock_t end = clock();
    double time = (double) (end - start) / CLOCKS_PER_SEC;
    double reductions_per_second = (double) REDUCTIONS / time;
    fprintf(stderr, "\n[Reductions: %zu, CPU time: %f, R/s: %f]\n", REDUCTIONS, time, reductions_per_second);
#endif

    return 0;
}}
"#,
        )?;

        Ok(())
    }
}
