//! 编译到 C 语言的运行时

use anyhow::Result;

use crate::{
    options::Options,
    runtime::{
        AgentId, AgentMeta, Function, FunctionMeta, Initializer, Instruction, Program, Rule,
    },
};

/// 编译到 C 语言的运行时
pub struct C;

impl super::Target for C {
    fn write(mut f: impl std::io::Write, program: Program, options: &Options) -> Result<()> {
        Self::write_includes(&mut f, options)?;
        Self::write_prelude(&mut f, options)?;
        Self::write_global(&mut f, program.agents)?;
        Self::write_runtime(&mut f)?;

        for rule in program.rules {
            Self::write_rule(&mut f, rule)?;
        }

        Self::write_rule_map(&mut f, program.rule_map)?;

        for function in program.functions {
            Self::write_function(&mut f, function)?;
        }
        Self::write_function_meta(&mut f, program.function_meta)?;
        Self::write_main(&mut f, program.entry_point)?;
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
typedef size_t* (*NetFun)();

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

    fn write_global(mut f: impl std::io::Write, agents: Vec<AgentMeta>) -> Result<()> {
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
#define NAME_COUNTER_START {agents_count}
const char* AGENTS[] = {{ {agents_names} }};
const size_t ARITY[] = {{ {agents_arity} }};
size_t NAME_COUNTER = NAME_COUNTER_START;
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
        fprintf(stderr, "\x1b[31merror\x1b[0m: stack overflow\n");
        fprintf(stderr, "\x1b[33mhint\x1b[0m: try to increase the stack size with `--stack-size`\n");
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

void free_term(size_t* term) {
    if (IS_NAME(term)) {
        free(term);
        return;
    }
    size_t arity = ARITY[term[0]];
    for (size_t i = 1; i <= arity; i++) {
        free_term((size_t*) term[i]);
    }
    free(term);
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
            fprintf(stderr, "\x1b[31merror\x1b[0m: no rule for ");
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

    fn write_rule(mut f: impl std::io::Write, rule: Rule) -> Result<()> {
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

    fn write_initializer(mut f: impl std::io::Write, initializer: Initializer) -> Result<()> {
        match initializer {
            Initializer::Name { index } => {
                writeln!(f, "    size_t* x{index} = new_name();")?;
            }
            Initializer::Agent { index, id } => {
                writeln!(f, "    size_t* a{index} = new_agent({id});")?
            }
            Initializer::SlotFromLeft { index, slot } => {
                writeln!(f, "    size_t* s{index} = (size_t*) left[{slot}];",)?
            }
            Initializer::SlotFromRight { index, slot } => {
                writeln!(f, "    size_t* s{index} = (size_t*) right[{slot}];",)?
            }
        }
        Ok(())
    }

    fn write_instruction(mut f: impl std::io::Write, instruction: Instruction) -> Result<()> {
        match instruction {
            Instruction::SetSlot {
                target,
                slot,
                value,
            } => writeln!(f, "    {target}[{slot}] = (size_t) {value};")?,
            Instruction::PushEquation {
                left,
                right,
                description,
            } => writeln!(f, "    push_equation({left}, {right});  // {description}")?,
        }
        Ok(())
    }

    fn write_rule_map(
        mut f: impl std::io::Write,
        rule_map: Vec<(AgentId, AgentId, usize)>,
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

    fn write_function(mut f: impl std::io::Write, func: Function) -> Result<()> {
        write!(
            f,
            r#"
size_t** func_{id}() {{
"#,
            id = func.index
        )?;

        for initializer in func.initializers {
            Self::write_initializer(&mut f, initializer)?;
        }
        for instruction in func.instructions {
            Self::write_instruction(&mut f, instruction)?;
        }

        writeln!(
            f,
            r#"
    size_t** outputs = malloc(sizeof(size_t*) * {count});
"#,
            count = func.outputs.len()
        )?;
        for (i, output) in func.outputs.into_iter().enumerate() {
            writeln!(f, r#"    outputs[{i}] = {output};"#)?;
        }

        write!(
            f,
            r#"
    return outputs;
}}
"#,
        )?;

        Ok(())
    }

    fn write_function_meta(
        mut f: impl std::io::Write,
        function_meta: Vec<FunctionMeta>,
    ) -> Result<()> {
        write!(
            f,
            r#"
const NetFun NET_FUNCS[] = {{ {} }};
const size_t OUTPUT_COUNTS[] = {{ {} }};
"#,
            (0..function_meta.len())
                .map(|i| format!("func_{}", i))
                .collect::<Vec<_>>()
                .join(", "),
            function_meta
                .into_iter()
                .map(|m| m.output_count.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )?;

        Ok(())
    }

    fn write_main(mut f: impl std::io::Write, entry_point: usize) -> Result<()> {
        write!(
            f,
            r#"
int main() {{
#ifdef ZZ_TIMING
    clock_t start = clock();
#endif

    size_t** outputs = NET_FUNCS[{entry_point}]();

    run();
    for (size_t i = 0; i < OUTPUT_COUNTS[{entry_point}]; i++) {{
        print_term(stdout, outputs[i], 1000);
        free_term(outputs[i]);
        printf("\n");
    }}
    free(outputs);

#ifdef ZZ_TIMING
    clock_t end = clock();
    double time = (double) (end - start) / CLOCKS_PER_SEC;
    double reductions_per_second = (double) REDUCTIONS / time;
    fprintf(stderr, "\n[Reductions: %zu, CPU time: %f, R/s: %f]\n", REDUCTIONS, time, reductions_per_second);
#endif

    return 0;
}}
"#
        )?;

        Ok(())
    }
}
