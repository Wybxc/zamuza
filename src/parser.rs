//! 语法解析器。

use crate::{
    ast::{self, Span},
    utils::lines_span,
};
use annotate_snippets::{
    display_list::{DisplayList, FormatOptions},
    snippet::{Annotation, AnnotationType, Slice, Snippet, SourceAnnotation},
};
use anyhow::Result;
use pest::{iterators::Pair, Parser};

mod grammar {
    #[derive(Parser)]
    #[grammar = "zamuza.pest"]
    pub struct ZamuzaParser;
}

use grammar::{Rule, ZamuzaParser};

/// 从文本生成抽象语法树
pub fn parse<'a>(source: &'a str, filename: &'a str) -> Result<ast::Program<'a>, String> {
    let mut parsed = ZamuzaParser::parse(Rule::Program, source).map_err(|err| {
        let (start, end) = match err.line_col {
            pest::error::LineColLocation::Pos(pos) => (pos, pos),
            pest::error::LineColLocation::Span(start, end) => (start, end),
        };
        let (line_range, range) = lines_span(source, start, end).unwrap_or_default();
        let message = err.variant.message();

        let snippet = Snippet {
            title: Some(Annotation {
                id: None,
                label: Some("syntax error"),
                annotation_type: AnnotationType::Error,
            }),
            footer: vec![],
            slices: vec![Slice {
                source: &source[line_range],
                line_start: start.0,
                origin: Some(filename),
                annotations: vec![SourceAnnotation {
                    range,
                    label: &message,
                    annotation_type: AnnotationType::Error,
                }],
                fold: true,
            }],
            opt: FormatOptions {
                color: true,
                ..Default::default()
            },
        };

        DisplayList::from(snippet).to_string()
    })?;
    let pairs = parsed.next().unwrap().into_inner();

    let mut rules = vec![];
    let mut equations = vec![];
    let mut interfaces = vec![];

    for pair in pairs {
        match pair.as_rule() {
            Rule::Rule => rules.push(parse_rule(pair)),
            Rule::Equation => equations.push(parse_equation(pair)),
            Rule::Interface => {
                interfaces.push(parse_interface(pair));
            }
            Rule::EOI => {}
            _ => unreachable!(),
        }
    }

    Ok(ast::Program {
        filename,
        source,
        rules,
        equations,
        interfaces,
    })
}

fn parse_rule(rule: Pair<'_, Rule>) -> Span<'_, ast::Rule<'_>> {
    let span = rule.as_span();
    let mut rule = rule.into_inner();

    let term_pair = parse_rule_term_pair(rule.next().unwrap());
    let equations = parse_rule_equations(rule.next().unwrap());

    let rule = ast::Rule {
        term_pair,
        equations,
    };
    Span::new(rule, span)
}

fn parse_rule_term_pair(term_pair: Pair<'_, Rule>) -> Span<'_, ast::RuleTermPair<'_>> {
    let span = term_pair.as_span();
    let term_pair = term_pair.into_inner().next().unwrap();

    let left_to_right = match term_pair.as_rule() {
        Rule::RuleTermLeftRight => true,
        Rule::RuleTermRightLeft => false,
        _ => unreachable!(),
    };

    let mut term_pair = term_pair.into_inner();
    let terms = (
        parse_rule_term(term_pair.next().unwrap()),
        parse_rule_term(term_pair.next().unwrap()),
    );

    let term_pair = if left_to_right {
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
    Span::new(term_pair, span)
}

fn parse_rule_term(term: Pair<'_, Rule>) -> Span<'_, ast::RuleTerm<'_>> {
    let span = term.as_span();
    let mut terms = term.into_inner();
    let head = terms.next().unwrap();
    let agent = parse_agent(head);
    let body = terms.map(parse_name).collect::<Vec<_>>();
    let term = ast::RuleTerm { agent, body };
    Span::new(term, span)
}

fn parse_rule_equations(equations: Pair<'_, Rule>) -> Vec<Span<'_, ast::Equation<'_>>> {
    let equations = equations.into_inner();
    equations.map(parse_equation).collect::<Vec<_>>()
}

fn parse_equation(equation: Pair<'_, Rule>) -> Span<'_, ast::Equation<'_>> {
    let span = equation.as_span();
    let mut terms = equation.into_inner();

    let terms = terms.next().unwrap();
    let right_to_left = match terms.as_rule() {
        Rule::EquationLeftRight => false,
        Rule::EquationRightLeft => true,
        _ => unreachable!(),
    };

    let mut terms = terms.into_inner();
    let terms = (
        parse_term(terms.next().unwrap()),
        parse_term(terms.next().unwrap()),
    );

    let equation = if !right_to_left {
        ast::Equation {
            left: terms.0,
            right: terms.1,
        }
    } else {
        ast::Equation {
            left: terms.1,
            right: terms.0,
        }
    };
    Span::new(equation, span)
}

fn parse_interface(interface: Pair<'_, Rule>) -> ast::Term<'_> {
    let term = interface.into_inner().next().unwrap();
    parse_term(term)
}

fn parse_term(term: Pair<'_, Rule>) -> ast::Term<'_> {
    let span = term.as_span();
    let mut terms = term.into_inner();
    let head = terms.next().unwrap();
    let term = match head.as_rule() {
        Rule::Name => ast::Term::Name(parse_name(head)),
        Rule::Agent => {
            let name = parse_agent(head);
            let body = terms.map(parse_term).collect::<Vec<_>>();
            let agent = ast::Agent { name, body };
            ast::Term::Agent(Span::new(agent, span))
        }
        _ => unreachable!(),
    };
    term
}

fn parse_name(name: Pair<'_, Rule>) -> Span<'_, ast::Name<'_>> {
    let name = name.into_inner().next().unwrap();
    let rule = name.as_rule();
    let span = name.as_span();

    let name = name.into_inner().next().unwrap();
    let name = Span::new(name.as_str(), name.as_span());
    let name = match rule {
        Rule::NameIn => ast::Name::In(name),
        Rule::NameOut => ast::Name::Out(name),
        _ => unreachable!(),
    };
    Span::new(name, span)
}

fn parse_agent(agent: Pair<'_, Rule>) -> Span<'_, &str> {
    Span::new(agent.as_str(), agent.as_span())
}
