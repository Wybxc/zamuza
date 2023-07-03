//! IR 优化

use super::{AgentId, Program, Rule, RuleInitializer, RuleInstruction};

/// 优化 IR
pub fn optimize(program: &mut Program) {
    for rule in program.rules.iter_mut() {
        optimize_new_free(rule, &program.rule_map);
    }
}

/// 优化规则中的重复申请/释放内存
pub fn optimize_new_free(rule: &mut Rule, rule_map: &[(AgentId, AgentId, usize)]) {
    let left_id = rule_map[rule.index].0;
    let right_id = rule_map[rule.index].1;

    if let Some((left_reuse_index, index)) =
        rule.initializers
            .iter()
            .enumerate()
            .find_map(|(i, x)| match x {
                RuleInitializer::Agent { index, id } if *id == left_id => Some((i, *index)),
                _ => None,
            })
    {
        if let Some(left_free_index) =
            rule.instructions
                .iter()
                .enumerate()
                .find_map(|(i, x)| match x {
                    RuleInstruction::FreeLeft => Some(i),
                    _ => None,
                })
        {
            rule.initializers.remove(left_reuse_index);
            rule.instructions.remove(left_free_index);
            rule.initializers.push(RuleInitializer::ReuseLeft { index });
        }
    }

    if let Some((right_reuse_index, index)) =
        rule.initializers
            .iter()
            .enumerate()
            .find_map(|(i, x)| match x {
                RuleInitializer::Agent { index, id } if *id == right_id => Some((i, *index)),
                _ => None,
            })
    {
        if let Some(right_free_index) =
            rule.instructions
                .iter()
                .enumerate()
                .find_map(|(i, x)| match x {
                    RuleInstruction::FreeRight => Some(i),
                    _ => None,
                })
        {
            rule.initializers.remove(right_reuse_index);
            rule.instructions.remove(right_free_index);
            rule.initializers
                .push(RuleInitializer::ReuseRight { index });
        }
    }
}
