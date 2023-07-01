//! 语义检查。

use std::collections::{HashMap, HashSet};

use thiserror::Error;

use crate::{
    ast::{self, Span},
    utils::lines_span,
};

/// 类型检查错误。
#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum TypeError<'a> {
    #[error("variable appears more than once in a rule")]
    NonLinearRule { name: &'a Span<'a, ast::Name<'a>> },

    #[error("variable appears {} times", .count)]
    VariableCountError {
        name: &'a Span<'a, ast::Name<'a>>,
        count: i32,
    },

    #[error("rules overlap")]
    OverlappingRules(
        &'a Span<'a, ast::RuleTermPair<'a>>,
        &'a Span<'a, ast::RuleTermPair<'a>>,
    ),

    #[error("input-output balance error")]
    MultipleTimesAsInput { name: &'a Span<'a, ast::Name<'a>> },

    #[error("input-output balance error")]
    MultipleTimesAsOutput { name: &'a Span<'a, ast::Name<'a>> },

    #[error("input-output balance error")]
    MisdirectedInput { name: &'a Span<'a, ast::Name<'a>> },

    #[error("input-output balance error")]
    MisdirectedOutput { name: &'a Span<'a, ast::Name<'a>> },

    #[error("no main function")]
    NoMainFunction,
}

impl<'a> TypeError<'a> {
    fn span(&self, source: &'a str) -> pest::Span<'a> {
        match self {
            TypeError::NonLinearRule { name } => name.span(),
            TypeError::VariableCountError { name, .. } => name.span(),
            TypeError::OverlappingRules(r1, r2) => {
                let start = r1.span().start().min(r2.span().start());
                let end = r1.span().end().max(r2.span().end());
                pest::Span::new(source, start, end).unwrap()
            }
            TypeError::MultipleTimesAsInput { name } => name.span(),
            TypeError::MultipleTimesAsOutput { name } => name.span(),
            TypeError::MisdirectedInput { name } => name.span(),
            TypeError::MisdirectedOutput { name } => name.span(),
            TypeError::NoMainFunction => pest::Span::new(source, 0, 0).unwrap(),
        }
    }

    fn annotations(&self) -> Vec<(pest::Span, &str, annotate_snippets::snippet::AnnotationType)> {
        use annotate_snippets::snippet::AnnotationType;
        match self {
            TypeError::NonLinearRule { name } => {
                vec![(name.span(), "appears more than once", AnnotationType::Error)]
            }
            TypeError::VariableCountError { name, .. } => {
                vec![(
                    name.span(),
                    "should appear exactly twice",
                    AnnotationType::Error,
                )]
            }
            TypeError::OverlappingRules(r1, r2) => vec![
                (r1.span(), "overlaps ...", AnnotationType::Error),
                (r2.span(), "with this rule", AnnotationType::Info),
            ],
            TypeError::MultipleTimesAsInput { name } => vec![(
                name.span(),
                "appears more than once as input",
                AnnotationType::Error,
            )],
            TypeError::MultipleTimesAsOutput { name } => vec![(
                name.span(),
                "appears more than once as output",
                AnnotationType::Error,
            )],
            TypeError::MisdirectedInput { name } => vec![(
                name.span(),
                "appears as input, where it should be output",
                AnnotationType::Error,
            )],
            TypeError::MisdirectedOutput { name } => vec![(
                name.span(),
                "appears as output, where it should be input",
                AnnotationType::Error,
            )],
            TypeError::NoMainFunction => vec![(
                pest::Span::new("", 0, 0).unwrap(),
                "no main function",
                AnnotationType::Error,
            )],
        }
    }

    /// 将错误转换为可供显示的字符串。
    pub fn to_snippet(&self, source: &str, filename: &str) -> String {
        use annotate_snippets::display_list::{DisplayList, FormatOptions};
        use annotate_snippets::snippet::{
            Annotation, AnnotationType, Slice, Snippet, SourceAnnotation,
        };

        let label = self.to_string();
        let span = self.span(source);
        let start = span.start_pos().line_col();
        let end = span.end_pos().line_col();
        let (line_range, _) = lines_span(source, start, end).unwrap_or_default();

        let snippet = Snippet {
            title: Some(Annotation {
                id: None,
                label: Some(label.as_str()),
                annotation_type: AnnotationType::Error,
            }),
            footer: vec![],
            slices: vec![Slice {
                source: &source[line_range.clone()],
                line_start: start.0,
                origin: Some(filename),
                annotations: self
                    .annotations()
                    .into_iter()
                    .map(|(range, label, ty)| SourceAnnotation {
                        range: (
                            range.start() - line_range.start,
                            range.end() - line_range.start,
                        ),
                        label,
                        annotation_type: ty,
                    })
                    .collect(),
                fold: true,
            }],
            opt: FormatOptions {
                color: true,
                ..Default::default()
            },
        };

        DisplayList::from(snippet).to_string()
    }
}

/// 规则项中，每个变量只能出现一次
pub fn check_rule_terms<'a>(rule: &'a ast::Rule) -> Result<(), TypeError<'a>> {
    let mut names = HashSet::new();
    for name in rule
        .term_pair
        .left
        .body
        .iter()
        .chain(rule.term_pair.right.body.iter())
    {
        if !names.insert(name.as_name()) {
            return Err(TypeError::NonLinearRule { name });
        }
    }
    Ok(())
}

fn count_names<'a>(
    term: &'a ast::Term,
    names: &mut HashMap<&'a str, (i32, Option<&'a Span<'a, ast::Name<'a>>>)>,
) {
    match term {
        ast::Term::Name(name) => {
            let entry = names.entry(name.as_name()).or_insert((0, None));
            entry.0 += 1;
            entry.1 = Some(name);
        }
        ast::Term::Agent(agent) => {
            for term in &agent.body {
                count_names(term, names);
            }
        }
    }
}

/// 规则中，所有变量必须恰好出现两次
pub fn check_rule_variables<'a>(rule: &'a ast::Rule) -> Result<(), TypeError<'a>> {
    let mut names = HashMap::new();
    for name in rule
        .term_pair
        .left
        .body
        .iter()
        .chain(rule.term_pair.right.body.iter())
    {
        let entry = names.entry(name.as_name()).or_insert((0, None));
        entry.0 += 1;
        entry.1 = Some(name);
    }
    for equation in &rule.equations {
        count_names(&equation.left, &mut names);
        count_names(&equation.right, &mut names);
    }

    for (_, (count, name)) in names {
        if count != 2 {
            let name = name.unwrap();
            return Err(TypeError::VariableCountError { name, count });
        }
    }
    Ok(())
}

/// 网络中，所有变量必须恰好出现两次
pub fn check_net_variables<'a>(net: &'a ast::Net) -> Result<(), TypeError<'a>> {
    let mut names = HashMap::new();
    for equation in &net.equations {
        count_names(&equation.left, &mut names);
        count_names(&equation.right, &mut names);
    }
    for interface in &net.interfaces {
        count_names(interface, &mut names);
    }

    for (_, (count, name)) in names {
        if count != 2 {
            let name = name.unwrap();
            return Err(TypeError::VariableCountError { name, count });
        }
    }
    Ok(())
}

/// 不允许冲突的规则
pub fn check_overlapping<'a>(program: &'a ast::Module) -> Result<(), TypeError<'a>> {
    let mut rule_sets: HashMap<_, &Span<ast::Rule>> = HashMap::new();
    for rule in &program.rules {
        let agents = (
            *rule.term_pair.left.agent.as_ref(),
            *rule.term_pair.right.agent.as_ref(),
        );
        if let Some(other) = rule_sets.get(&agents) {
            return Err(TypeError::OverlappingRules(
                &rule.term_pair,
                &other.term_pair,
            ));
        }
        rule_sets.insert(agents, rule);
    }
    Ok(())
}

fn check_term_io_balance<'a>(
    term: &'a ast::Term,
    input_map: &mut HashMap<&'a str, bool>,
) -> Result<(), TypeError<'a>> {
    match term {
        ast::Term::Name(name) => {
            if let Some(&input) = input_map.get(name.as_name()) {
                match name.as_ref() {
                    ast::Name::In(_) if input => {
                        return Err(TypeError::MultipleTimesAsInput { name })
                    }
                    ast::Name::Out(_) if !input => {
                        return Err(TypeError::MultipleTimesAsOutput { name })
                    }
                    _ => {}
                };
            } else {
                match name.as_ref() {
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
    net: &'a [Span<ast::Equation>],
    mut input_map: HashMap<&'a str, bool>,
) -> Result<(), TypeError<'a>> {
    for equation in net {
        check_term_io_balance(&equation.left, &mut input_map)?;
        check_term_io_balance(&equation.right, &mut input_map)?;
    }
    Ok(())
}

/// 网络输入-输出平衡
pub fn check_net_io_balance<'a>(net: &'a ast::Net) -> Result<(), TypeError<'a>> {
    let mut input_map = HashMap::new();
    for interface in &net.interfaces {
        check_term_io_balance(interface, &mut input_map)?;
    }
    check_equations_io_balance(&net.equations, input_map)
}

/// 规则输入-输出平衡
pub fn check_rule_io_balance<'a>(rule: &'a ast::Rule) -> Result<(), TypeError<'a>> {
    let mut input_map = HashMap::new();
    for name in rule
        .term_pair
        .left
        .body
        .iter()
        .chain(rule.term_pair.right.body.iter())
    {
        match name.as_ref() {
            // Write the opposite input flag
            ast::Name::In(_) => input_map.insert(name.as_name(), false),
            ast::Name::Out(_) => input_map.insert(name.as_name(), true),
        };
    }
    check_equations_io_balance(&rule.equations, input_map)
}

/// 变量：左输入右输出
pub fn check_equation_var_io_dir<'a>(equation: &'a ast::Equation) -> Result<(), TypeError<'a>> {
    if let ast::Term::Name(name) = &equation.left {
        if let ast::Name::Out(_) = name.as_ref() {
            return Err(TypeError::MisdirectedOutput { name });
        }
    }
    if let ast::Term::Name(name) = &equation.right {
        if let ast::Name::In(_) = name.as_ref() {
            return Err(TypeError::MisdirectedInput { name });
        }
    }
    Ok(())
}

/// Main 函数存在
pub fn check_main<'a>(program: &'a ast::Module) -> Result<(), TypeError<'a>> {
    if !program.nets.iter().any(|net| *net.name.as_ref() == "Main") {
        return Err(TypeError::NoMainFunction);
    }
    Ok(())
}

/// 检查整个程序
pub fn check_module<'a>(module: &'a ast::Module) -> Result<(), TypeError<'a>> {
    for rule in &module.rules {
        check_rule_terms(rule)?;
        check_rule_variables(rule)?;
        check_rule_io_balance(rule)?;
        for equation in &rule.equations {
            check_equation_var_io_dir(equation)?;
        }
    }

    for net in &module.nets {
        check_net_variables(net)?;
        check_net_io_balance(net)?;
        for equation in &net.equations {
            check_equation_var_io_dir(equation)?;
        }
    }

    check_overlapping(module)?;
    check_main(module)?;
    Ok(())
}
