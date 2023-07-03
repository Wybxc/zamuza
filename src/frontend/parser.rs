//! 语法解析器。

use crate::{frontend::ast, utils::Span};
use annotate_snippets::{
    display_list::{DisplayList, FormatOptions},
    snippet::{Annotation, AnnotationType, Snippet},
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
pub fn parse<'a>(source: &'a str, filename: &'a str) -> Result<Span<'a, ast::Module<'a>>, String> {
    let mut parsed = ZamuzaParser::parse(Rule::Program, source).map_err(|err| {
        let (start, end) = match err.location {
            pest::error::InputLocation::Pos(pos) => (pos, pos),
            pest::error::InputLocation::Span(span) => span,
        };
        let lines = Span::new((), filename, source, start, end)
            .lines()
            .unwrap_or_default();
        let message = err.variant.message();

        let snippet = Snippet {
            title: Some(Annotation {
                id: None,
                label: Some("syntax error"),
                annotation_type: AnnotationType::Error,
            }),
            footer: vec![],
            slices: vec![lines.as_annotation(&message, AnnotationType::Error)],
            opt: FormatOptions {
                color: true,
                ..Default::default()
            },
        };

        DisplayList::from(snippet).to_string()
    })?;

    Ok(ModuleParser { filename, source }.parse_module(parsed.next().unwrap()))
}

#[derive(Copy, Clone)] // 让 self 是 Copy 的，下面少引入一个生命周期（实在是被生命周期搞烦了）
struct ModuleParser<'a> {
    filename: &'a str,
    source: &'a str,
}

impl<'a> ModuleParser<'a> {
    fn parse_module(self, module: Pair<'a, Rule>) -> Span<'a, ast::Module<'a>> {
        let span = module.as_span();
        let pairs = module.into_inner();

        let mut rules = vec![];
        let mut nets = vec![];

        for pair in pairs {
            match pair.as_rule() {
                Rule::Rule => rules.push(self.parse_rule(pair)),
                Rule::Net => nets.push(self.parse_net(pair)),
                Rule::EOI => {}
                _ => unreachable!(),
            }
        }

        let module = ast::Module { rules, nets };
        Span::from_pest(module, self.filename, self.source, span)
    }

    fn parse_rule(self, rule: Pair<'a, Rule>) -> Span<'a, ast::Rule<'a>> {
        let span = rule.as_span();
        let mut rule = rule.into_inner();

        let term_pair = self.parse_rule_term_pair(rule.next().unwrap());
        let equations = self.parse_rule_equations(rule.next().unwrap());

        let rule = ast::Rule {
            term_pair,
            equations,
        };
        Span::from_pest(rule, self.filename, self.source, span)
    }

    fn parse_rule_term_pair(self, term_pair: Pair<'a, Rule>) -> Span<'a, ast::RuleTermPair<'a>> {
        let span = term_pair.as_span();
        let term_pair = term_pair.into_inner().next().unwrap();

        let left_to_right = match term_pair.as_rule() {
            Rule::RuleTermLeftRight => true,
            Rule::RuleTermRightLeft => false,
            _ => unreachable!(),
        };

        let mut term_pair = term_pair.into_inner();
        let terms = (
            self.parse_rule_term(term_pair.next().unwrap()),
            self.parse_rule_term(term_pair.next().unwrap()),
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
        Span::from_pest(term_pair, self.filename, self.source, span)
    }

    fn parse_rule_term(self, term: Pair<'a, Rule>) -> Span<'a, ast::RuleTerm<'a>> {
        let span = term.as_span();
        let mut terms = term.into_inner();
        let head = terms.next().unwrap();
        let agent = self.parse_ident(head);
        let body = terms.map(|p| self.parse_name(p)).collect::<Vec<_>>();
        let term = ast::RuleTerm { agent, body };
        Span::from_pest(term, self.filename, self.source, span)
    }

    fn parse_rule_equations(self, equations: Pair<'a, Rule>) -> Vec<Span<'a, ast::Equation<'a>>> {
        let equations = equations.into_inner();
        equations
            .map(|x| self.parse_equation(x))
            .collect::<Vec<_>>()
    }

    fn parse_net(self, net: Pair<'a, Rule>) -> Span<'a, ast::Net<'a>> {
        let span = net.as_span();
        let mut net = net.into_inner();

        let name = self.parse_ident(net.next().unwrap());
        let interfaces = self.parse_interfaces(net.next().unwrap());
        let equations = self.parse_net_equations(net.next().unwrap());

        let net = ast::Net {
            name,
            interfaces,
            equations,
        };
        Span::from_pest(net, self.filename, self.source, span)
    }

    fn parse_interfaces(self, interfaces: Pair<'a, Rule>) -> Vec<ast::Term<'a>> {
        let interfaces = interfaces.into_inner();
        interfaces.map(|x| self.parse_term(x)).collect::<Vec<_>>()
    }

    fn parse_net_equations(self, equations: Pair<'a, Rule>) -> Vec<Span<'a, ast::Equation<'a>>> {
        let equations = equations.into_inner();
        equations
            .map(|x| self.parse_equation(x))
            .collect::<Vec<_>>()
    }

    fn parse_equation(self, equation: Pair<'a, Rule>) -> Span<'a, ast::Equation<'a>> {
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
            self.parse_term(terms.next().unwrap()),
            self.parse_term(terms.next().unwrap()),
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
        Span::from_pest(equation, self.filename, self.source, span)
    }

    fn parse_term(self, term: Pair<'a, Rule>) -> ast::Term<'a> {
        let span = term.as_span();
        let mut terms = term.into_inner();
        let head = terms.next().unwrap();
        let term = match head.as_rule() {
            Rule::Name => ast::Term::Name(self.parse_name(head)),
            Rule::Agent => {
                let name = self.parse_ident(head);
                let body = terms.map(|x| self.parse_term(x)).collect::<Vec<_>>();
                let agent = ast::Agent { name, body };
                ast::Term::Agent(Span::from_pest(agent, self.filename, self.source, span))
            }
            _ => unreachable!(),
        };
        term
    }

    fn parse_name(self, name: Pair<'a, Rule>) -> Span<'a, ast::Name<'a>> {
        let name = name.into_inner().next().unwrap();
        let rule = name.as_rule();
        let span = name.as_span();

        let name = name.into_inner().next().unwrap();
        let name = Span::from_pest(name.as_str(), self.filename, self.source, name.as_span());
        let name = match rule {
            Rule::NameIn => ast::Name::In(name),
            Rule::NameOut => ast::Name::Out(name),
            _ => unreachable!(),
        };
        Span::from_pest(name, self.filename, self.source, span)
    }

    fn parse_ident(self, agent: Pair<'a, Rule>) -> Span<'a, &str> {
        Span::from_pest(agent.as_str(), self.filename, self.source, agent.as_span())
    }
}
