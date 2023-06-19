use crate::ast;
use anyhow::{anyhow, bail, Result};
use pest::{iterators::Pair, Parser};

#[derive(Parser)]
#[grammar = "zamuza.pest"]
pub struct ZamuzaParser;

pub fn parse(input: &str) -> Result<ast::Program> {
    let mut parsed = ZamuzaParser::parse(Rule::Program, input)?;
    let pairs = parsed.next().unwrap().into_inner();

    let mut rules = vec![];
    let mut equations = vec![];
    let mut interface = None;

    for pair in pairs {
        match pair.as_rule() {
            Rule::Rule => rules.push(parse_rule(pair)?),
            Rule::Equation => equations.push(parse_equation(pair)?),
            Rule::Interface => {
                if interface.is_some() {
                    bail!("Interface can only be defined once");
                }
                interface = Some(parse_interface(pair)?);
            }
            Rule::EOI => {}
            _ => unreachable!(),
        }
    }

    let interface = interface.ok_or_else(|| anyhow!("The program must have an interface"))?;
    Ok(ast::Program {
        rules,
        equations,
        interface,
    })
}

fn parse_rule(rule: Pair<Rule>) -> Result<ast::Rule> {
    let mut rule = rule.into_inner();
    let terms = [
        parse_rule_term(rule.next().unwrap())?,
        parse_rule_term(rule.next().unwrap())?,
    ];
    let equations = parse_rule_equations(rule.next().unwrap())?;
    Ok(ast::Rule { terms, equations })
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
    let left = terms.next().unwrap();
    let right = terms.next().unwrap();
    Ok(ast::Equation {
        left: parse_term(left)?,
        right: parse_term(right)?,
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
    ast::Name(name.into_inner().as_str().to_string())
}

fn parse_agent(agent: Pair<Rule>) -> String {
    agent.as_str().to_string()
}
