// SPDX-License-Identifier: GPL-3.0-or-later
//! What a file of texts holds, for the tests that keep the languages in step
//! (docs/specs/translations.md §4): each text's id and the arguments it uses.

use std::collections::{BTreeMap, BTreeSet};

use fluent_syntax::ast::{Entry, Expression, InlineExpression, Pattern, PatternElement};
use fluent_syntax::parser;

/// The texts of one `.ftl` file: id -> the names of the arguments it uses.
#[derive(Debug, Default)]
pub struct FileTexts {
    pub texts: BTreeMap<String, BTreeSet<String>>,
    /// Lines that don't parse, texts given twice, texts without a value, attributes.
    pub errors: Vec<String>,
}

/// Read one `.ftl` file.
pub fn texts(source: &str) -> FileTexts {
    let mut out = FileTexts::default();
    let resource = match parser::parse(source) {
        Ok(r) => r,
        Err((r, errors)) => {
            out.errors
                .extend(errors.iter().map(|e| format!("does not parse: {e:?}")));
            r
        }
    };
    for entry in resource.body {
        match entry {
            Entry::Message(message) => {
                let id = message.id.name.to_string();
                if !message.attributes.is_empty() {
                    out.errors.push(format!("{id}: attributes are not used"));
                }
                let Some(value) = message.value else {
                    out.errors.push(format!("{id}: no text"));
                    continue;
                };
                let mut args = BTreeSet::new();
                pattern_args(&value, &mut args);
                if out.texts.insert(id.clone(), args).is_some() {
                    out.errors.push(format!("{id}: given twice"));
                }
            }
            Entry::Junk { content } => out.errors.push(format!("does not parse: {content}")),
            Entry::Term(_)
            | Entry::Comment(_)
            | Entry::GroupComment(_)
            | Entry::ResourceComment(_) => {}
        }
    }
    out
}

fn pattern_args(pattern: &Pattern<&str>, args: &mut BTreeSet<String>) {
    for element in &pattern.elements {
        if let PatternElement::Placeable { expression } = element {
            expression_args(expression, args);
        }
    }
}

fn expression_args(expression: &Expression<&str>, args: &mut BTreeSet<String>) {
    match expression {
        Expression::Inline(inline) => inline_args(inline, args),
        Expression::Select { selector, variants } => {
            inline_args(selector, args);
            for variant in variants {
                pattern_args(&variant.value, args);
            }
        }
    }
}

fn inline_args(inline: &InlineExpression<&str>, args: &mut BTreeSet<String>) {
    match inline {
        InlineExpression::VariableReference { id } => {
            args.insert(id.name.to_string());
        }
        InlineExpression::FunctionReference { arguments, .. } => {
            for a in &arguments.positional {
                inline_args(a, args);
            }
            for a in &arguments.named {
                inline_args(&a.value, args);
            }
        }
        InlineExpression::TermReference {
            arguments: Some(arguments),
            ..
        } => {
            for a in &arguments.positional {
                inline_args(a, args);
            }
            for a in &arguments.named {
                inline_args(&a.value, args);
            }
        }
        InlineExpression::Placeable { expression } => expression_args(expression, args),
        InlineExpression::StringLiteral { .. }
        | InlineExpression::NumberLiteral { .. }
        | InlineExpression::MessageReference { .. }
        | InlineExpression::TermReference { .. } => {}
    }
}
