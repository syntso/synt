use std::ops::ControlFlow;

use sqlparser::ast::{Expr, ObjectName, ObjectNamePart, Query, SetExpr, Statement, Visitor};

use super::risky::is_risky;
use crate::Effect;

#[derive(Debug, Default)]
pub(super) struct Effects {
    statements: usize,
    effect: Option<Effect>,
}

impl Effects {
    pub(super) fn found(&self) -> Option<Effect> {
        self.effect
    }

    fn raise(&mut self, effect: Effect) {
        self.effect = self.effect.max(Some(effect));
    }

    fn check_name(&mut self, name: &ObjectName) {
        let risky = match name.0.last() {
            Some(ObjectNamePart::Identifier(last)) => is_risky(&last.value),
            _ => true,
        };

        if risky {
            self.raise(Effect::Privileged);
        }
    }
}

impl Visitor for Effects {
    type Break = ();

    fn pre_visit_statement(&mut self, statement: &Statement) -> ControlFlow<()> {
        self.statements += 1;

        if self.statements == 1 {
            return ControlFlow::Continue(());
        }

        match statement {
            Statement::Query(_) => {}
            Statement::Insert(_)
            | Statement::Update(_)
            | Statement::Delete(_)
            | Statement::Merge(_) => self.raise(Effect::Write),
            _ => self.raise(Effect::Privileged),
        }

        ControlFlow::Continue(())
    }

    fn pre_visit_query(&mut self, query: &Query) -> ControlFlow<()> {
        if !query.locks.is_empty() {
            self.raise(Effect::Write);
        }

        if selects_into(&query.body) {
            self.raise(Effect::Privileged);
        }

        ControlFlow::Continue(())
    }

    fn pre_visit_relation(&mut self, relation: &ObjectName) -> ControlFlow<()> {
        self.check_name(relation);

        ControlFlow::Continue(())
    }

    fn pre_visit_expr(&mut self, expression: &Expr) -> ControlFlow<()> {
        if let Expr::Function(function) = expression {
            self.check_name(&function.name);
        }

        ControlFlow::Continue(())
    }
}

fn selects_into(body: &SetExpr) -> bool {
    match body {
        SetExpr::Select(select) => select.into.is_some(),
        SetExpr::SetOperation { left, right, .. } => selects_into(left) || selects_into(right),
        _ => false,
    }
}
