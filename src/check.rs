//! 语义检查。

use std::collections::{HashMap, HashSet};

use anyhow::{bail, Result};

use crate::ast;

/// 规则项中，每个变量只能出现一次
pub fn check_rule_terms(rule: &ast::Rule) -> Result<()> {
    let mut names = HashSet::new();
    for name in rule
        .term_pair
        .left
        .body
        .iter()
        .chain(rule.term_pair.right.body.iter())
    {
        if !names.insert(name.as_name()) {
            bail!("variable `{}` appears more than once", name);
        }
    }
    Ok(())
}

fn count_names<'a>(term: &'a ast::Term, names: &mut HashMap<&'a str, i32>) {
    match term {
        ast::Term::Name(name) => {
            *names.entry(name.as_name()).or_insert(0) += 1;
        }
        ast::Term::Agent(agent) => {
            for term in &agent.body {
                count_names(term, names);
            }
        }
    }
}

/// 规则中，所有变量必须恰好出现两次
pub fn check_rule_variables(rule: &ast::Rule) -> Result<()> {
    let mut names = HashMap::new();
    for name in rule
        .term_pair
        .left
        .body
        .iter()
        .chain(rule.term_pair.right.body.iter())
    {
        *names.entry(name.as_name()).or_insert(0) += 1;
    }
    for equation in &rule.equations {
        count_names(&equation.left, &mut names);
        count_names(&equation.right, &mut names);
    }

    for (name, count) in names {
        if count != 2 {
            bail!("variable `{}` appears {} times", name, count);
        }
    }
    Ok(())
}

/// 网络中，所有变量必须恰好出现两次
pub fn check_net_variables(program: &ast::Program) -> Result<()> {
    let mut names = HashMap::new();
    for interface in &program.interfaces {
        count_names(interface, &mut names);
    }
    for equation in &program.net {
        count_names(&equation.left, &mut names);
        count_names(&equation.right, &mut names);
    }

    for (name, count) in names {
        if count != 2 {
            bail!("variable `{}` appears {} times", name, count);
        }
    }
    Ok(())
}

/// 不允许冲突的规则
pub fn check_conflict(program: &ast::Program) -> Result<()> {
    let mut rule_sets = HashMap::new();
    for rule in &program.rules {
        let agents = (
            rule.term_pair.left.agent.as_str(),
            rule.term_pair.right.agent.as_str(),
        );
        if let Some(other) = rule_sets.get(&agents) {
            bail!("conflict rules:\n{}\n{}", other, rule);
        }
        rule_sets.insert(agents, rule);
    }
    Ok(())
}

fn check_term_io_balance<'a>(
    term: &'a ast::Term,
    input_map: &mut HashMap<&'a str, bool>,
) -> Result<()> {
    match term {
        ast::Term::Name(name) => {
            if let Some(&input) = input_map.get(name.as_name()) {
                match name {
                    ast::Name::In(_) if input => {
                        bail!("variable `{}` is used as input more than once", name)
                    }
                    ast::Name::Out(_) if !input => {
                        bail!("variable `{}` is used as output more than once", name)
                    }
                    _ => {}
                };
            } else {
                match name {
                    ast::Name::In(_) => input_map.insert(name.as_name(), true),
                    ast::Name::Out(_) => input_map.insert(name.as_name(), false),
                };
            }
        }
        ast::Term::Agent(agent) => {
            for term in &agent.body {
                check_term_io_balance(term, input_map)?;
            }
        }
    }
    Ok(())
}

fn check_equations_io_balance<'a>(
    net: &'a [ast::Equation],
    mut input_map: HashMap<&'a str, bool>,
) -> Result<()> {
    for equation in net {
        check_term_io_balance(&equation.left, &mut input_map)?;
        check_term_io_balance(&equation.right, &mut input_map)?;
    }
    Ok(())
}

/// 网络输入-输出平衡
pub fn check_net_io_balanse(program: &ast::Program) -> Result<()> {
    let mut input_map = HashMap::new();
    for interface in &program.interfaces {
        check_term_io_balance(interface, &mut input_map)?;
    }
    check_equations_io_balance(&program.net, input_map)
}

/// 规则输入-输出平衡
pub fn check_rule_io_balance(rule: &ast::Rule) -> Result<()> {
    let mut input_map = HashMap::new();
    for name in rule
        .term_pair
        .left
        .body
        .iter()
        .chain(rule.term_pair.right.body.iter())
    {
        match name {
            // Write the opposite input flag
            ast::Name::In(_) => input_map.insert(name.as_name(), false),
            ast::Name::Out(_) => input_map.insert(name.as_name(), true),
        };
    }
    check_equations_io_balance(&rule.equations, input_map)
}

/// 变量：左输入右输出
pub fn check_equation_var_io_dir(equation: &ast::Equation) -> Result<()> {
    if let ast::Term::Name(ast::Name::Out(_)) = &equation.left {
        bail!("left side of equation cannot be output variable");
    }
    if let ast::Term::Name(ast::Name::In(_)) = &equation.right {
        bail!("right side of equation cannot be input variable");
    }
    Ok(())
}

/// 检查整个程序
pub fn check_program(program: &ast::Program) -> Result<()> {
    for rule in &program.rules {
        let fmt_err = |e| anyhow::anyhow!("{} (in rule `{}`)", e, rule);
        check_rule_terms(rule).map_err(fmt_err)?;
        check_rule_variables(rule).map_err(fmt_err)?;
        check_rule_io_balance(rule).map_err(fmt_err)?;
        for equation in &rule.equations {
            check_equation_var_io_dir(equation).map_err(fmt_err)?;
        }
    }
    for equation in &program.net {
        let fmt_err = |e| anyhow::anyhow!("{} (in equation `{}`)", e, equation);
        check_equation_var_io_dir(equation).map_err(fmt_err)?;
    }

    check_net_variables(program)?;
    check_net_io_balanse(program)?;
    check_conflict(program)?;
    Ok(())
}
