mod binding;
mod chart_kind;
mod direction;
mod field;
mod form_child;
mod input;
mod key_value;
mod list_item;
mod series;
mod style;
mod submit;

use serde::Deserialize;
use serde_json::Value;

use binding::Binding;
use chart_kind::ChartKind;
use direction::Direction;
use field::Field;
use form_child::FormChild;
use input::Input;
use key_value::KeyValue;
use list_item::ListItem;
use series::Series;
use style::Style;
use submit::Submit;

use super::check::ViewCheck;
use crate::error::ViewError;

const MOST_LINES: usize = 5_000;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(super) enum ViewNode {
    Stack {
        #[serde(default, rename = "direction")]
        _direction: Option<Direction>,
        children: Vec<ViewNode>,
    },
    Text {
        #[serde(rename = "text")]
        _text: String,

        #[serde(default, rename = "style")]
        _style: Option<Style>,
    },
    Log {
        lines: Vec<String>,
    },
    List {
        items: Vec<ListItem>,
    },
    Table {
        columns: Vec<String>,
        rows: Vec<Vec<Value>>,
    },
    Keyvalue {
        items: Vec<KeyValue>,
    },
    Field {
        #[serde(flatten)]
        _field: Field,
    },
    Form {
        children: Vec<FormChild>,
        submit: Submit,
    },
    Button(Binding),
    Chart {
        #[serde(rename = "kind")]
        _kind: ChartKind,

        #[serde(rename = "series")]
        _series: Vec<Series>,
    },
}

impl ViewNode {
    pub(super) fn parse(value: &Value) -> Result<Self, ViewError> {
        Ok(Self::deserialize(value)?)
    }

    pub(super) fn check(&self, census: &mut ViewCheck<'_>, depth: usize) -> Result<(), ViewError> {
        census.count(depth)?;

        match self {
            Self::Stack { children, .. } => children
                .iter()
                .try_for_each(|child| child.check(census, depth + 1)),
            Self::Log { lines } => Self::check_log(lines),
            Self::List { items } => items.iter().try_for_each(|item| item.check(census)),
            Self::Table { columns, rows } => Self::check_table(columns, rows),
            Self::Keyvalue { items } => items.iter().try_for_each(KeyValue::check),
            Self::Form { children, submit } => {
                for _ in children {
                    census.count(depth + 1)?;
                }

                submit.binding().check(census)
            }
            Self::Button(binding) => binding.check(census),
            Self::Text { .. } | Self::Field { .. } | Self::Chart { .. } => Ok(()),
        }
    }

    fn check_log(lines: &[String]) -> Result<(), ViewError> {
        if lines.len() > MOST_LINES {
            return Err(ViewError::LongLog(MOST_LINES));
        }

        Ok(())
    }

    fn check_table(columns: &[String], rows: &[Vec<Value>]) -> Result<(), ViewError> {
        let plain = |cells: &Vec<Value>| {
            cells.len() == columns.len()
                && cells
                    .iter()
                    .all(|cell| !cell.is_array() && !cell.is_object())
        };

        if !rows.iter().all(plain) {
            return Err(ViewError::RaggedTable);
        }

        Ok(())
    }
}
