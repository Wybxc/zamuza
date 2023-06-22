//! 语法解析器。

use crate::ast;
use anyhow::Result;
use pest::{iterators::Pair, Parser};

mod grammar {
    #[derive(Parser)]
    #[grammar = "zamuza.pest"]
    pub struct ZamuzaParser;
}

use grammar::{Rule, ZamuzaParser};

/// 从文本生成抽象语法树
pub fn parse(input: &str) -> Result<ast::Program> {
    let mut parsed = ZamuzaParser::parse(Rule::Program, input)?;
    let pairs = parsed.next().unwrap().into_inner();

    let mut rules = vec![];
    let mut equations = vec![];
    let mut interfaces = vec![];

    for pair in pairs {
        match pair.as_rule() {
            Rule::Rule => rules.push(parse_rule(pair)?),
            Rule::Equation => equations.push(parse_equation(pair)?),
            Rule::Interface => {
                interfaces.push(parse_interface(pair)?);
            }
            Rule::EOI => {}
            _ => unreachable!(),
        }
    }

    Ok(ast::Program {
        rules,
        net: equations,
        interfaces,
    })
}

fn parse_rule(rule: Pair<Rule>) -> Result<ast::Rule> {
    let mut rule = rule.into_inner();

    let terms = rule.next().unwrap();
    let right_to_left = match terms.as_rule() {
        Rule::RuleTermsLeftRight => false,
        Rule::RuleTermsRightLeft => true,
        _ => unreachable!(),
    };

    let mut terms = terms.into_inner();
    let terms = (
        parse_rule_term(terms.next().unwrap())?,
        parse_rule_term(terms.next().unwrap())?,
    );

    let equations = parse_rule_equations(rule.next().unwrap())?;

    let term_pair = if !right_to_left {
        ast::RuleTermPair {
            left: terms.0,
            right: terms.1,
        }
    } else {
        ast::RuleTermPair {
            left: terms.1,
            right: terms.0,
        }
    };
    Ok(ast::Rule {
        term_pair,
        equations,
    })
}

fn parse_rule_term(term: Pair<Rule>) -> Result<ast::RuleTerm> {
    let mut terms = term.into_inner();
    let head = terms.next().unwrap();
    let agent = parse_agent(head);
    let terms = terms.map(parse_name).collect::<Vec<_>>();
    Ok(ast::RuleTerm { agent, body: terms })
}

fn parse_rule_equations(equations: Pair<Rule>) -> Result<Vec<ast::Equation>> {
    let equations = equations.into_inner();
    equations.map(parse_equation).collect::<Result<Vec<_>>>()
}

fn parse_equation(equation: Pair<Rule>) -> Result<ast::Equation> {
    let mut terms = equation.into_inner();

    let terms = terms.next().unwrap();
    let right_to_left = match terms.as_rule() {
        Rule::EquationLeftRight => false,
        Rule::EquationRightLeft => true,
        _ => unreachable!(),
    };

    let mut terms = terms.into_inner();
    let terms = (
        parse_term(terms.next().unwrap())?,
        parse_term(terms.next().unwrap())?,
    );

    Ok(if !right_to_left {
        ast::Equation {
            left: terms.0,
            right: terms.1,
        }
    } else {
        ast::Equation {
            left: terms.1,
            right: terms.0,
        }
    })
}

fn parse_interface(interface: Pair<Rule>) -> Result<ast::Term> {
    let term = interface.into_inner().next().unwrap();
    parse_term(term)
}

fn parse_term(term: Pair<Rule>) -> Result<ast::Term> {
    let mut terms = term.into_inner();
    let head = terms.next().unwrap();
    match head.as_rule() {
        Rule::Name => Ok(ast::Term::Name(parse_name(head))),
        Rule::Agent => {
            let agent = parse_agent(head);
            let terms = terms.map(parse_term).collect::<Result<Vec<_>>>()?;
            Ok(ast::Term::Agent(ast::Agent {
                name: agent,
                body: terms,
            }))
        }
        _ => unreachable!(),
    }
}

fn parse_name(name: Pair<Rule>) -> ast::Name {
    let name = name.into_inner().next().unwrap();
    let rule = name.as_rule();
    let name = name.into_inner().next().unwrap().as_str().to_string();
    match rule {
        Rule::NameIn => ast::Name::In(name),
        Rule::NameOut => ast::Name::Out(name),
        _ => unreachable!(),
    }
}

fn parse_agent(agent: Pair<Rule>) -> String {
    agent.as_str().to_string()
}
