use sqlparser::ast::{CopySource, CopyTarget, ObjectName, SetExpr, Statement, Visit};

use super::effects::Effects;
use crate::Effect;
use crate::statement::classification::Classification;
use crate::statement::kind::StatementKind;

const INFO_PRAGMAS: [&str; 9] = [
    "table_info",
    "table_xinfo",
    "table_list",
    "index_list",
    "index_info",
    "index_xinfo",
    "foreign_key_list",
    "foreign_key_check",
    "integrity_check",
];

pub(super) struct SingleStatement<'a> {
    statement: &'a Statement,
}

impl<'a> SingleStatement<'a> {
    pub(super) fn new(statement: &'a Statement) -> Self {
        Self { statement }
    }

    pub(super) fn classification(&self) -> Classification {
        Classification::new(self.effect(), self.kind())
    }

    fn effect(&self) -> Effect {
        let own = self.own_effect();

        let mut effects = Effects::default();
        let _visited = self.statement.visit(&mut effects);

        effects.found().map_or(own, |found| found.max(own))
    }

    fn own_effect(&self) -> Effect {
        match self.statement {
            Statement::Query(_)
            | Statement::Explain { analyze: false, .. }
            | Statement::ExplainTable { .. }
            | Statement::ShowFunctions { .. }
            | Statement::ShowVariable { .. }
            | Statement::ShowStatus { .. }
            | Statement::ShowVariables { .. }
            | Statement::ShowCreate { .. }
            | Statement::ShowColumns { .. }
            | Statement::ShowDatabases { .. }
            | Statement::ShowSchemas { .. }
            | Statement::ShowCharset(_)
            | Statement::ShowTables { .. }
            | Statement::ShowViews { .. }
            | Statement::ShowCollation { .. } => Effect::Read,
            Statement::Pragma { name, value, .. } => pragma_effect(name, value.is_some()),
            Statement::Explain {
                analyze: true,
                statement,
                ..
            } => SingleStatement::new(statement).own_effect(),
            Statement::Insert(_)
            | Statement::Update(_)
            | Statement::Delete(_)
            | Statement::Merge(_)
            | Statement::Copy {
                to: false,
                source: CopySource::Table { .. },
                target: CopyTarget::Stdin,
                ..
            } => Effect::Write,
            _ => Effect::Privileged,
        }
    }

    fn kind(&self) -> StatementKind {
        let known = match self.statement {
            Statement::Query(query) => match query.body.as_ref() {
                SetExpr::Insert(_) => "insert",
                SetExpr::Update(_) => "update",
                SetExpr::Delete(_) => "delete",
                SetExpr::Merge(_) => "merge",
                _ => "select",
            },
            Statement::Insert(_) => "insert",
            Statement::Update(_) => "update",
            Statement::Delete(_) => "delete",
            Statement::Merge(_) => "merge",
            Statement::Copy { .. } => "copy",
            Statement::Explain { .. } => "explain",
            Statement::ExplainTable { .. } => "describe",
            Statement::Pragma { .. } => "pragma",
            _ => return self.first_word(),
        };

        StatementKind::named(known)
    }

    fn first_word(&self) -> StatementKind {
        let text = self.statement.to_string();

        text.split_whitespace()
            .next()
            .map_or(StatementKind::Unparsable, |word| {
                StatementKind::named(word.to_ascii_lowercase())
            })
    }
}

fn pragma_effect(name: &ObjectName, sets_value: bool) -> Effect {
    let name = name.to_string().to_ascii_lowercase();
    let name = name.rsplit('.').next().unwrap_or_default();

    if !sets_value || INFO_PRAGMAS.contains(&name) {
        return Effect::Read;
    }

    Effect::Privileged
}
