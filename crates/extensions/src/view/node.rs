use serde_json::{Map, Value};

const MOST_LINES: usize = 5_000;

const STYLES: [&str; 6] = ["title", "body", "muted", "mono", "error", "success"];
const INPUTS: [&str; 5] = ["text", "number", "toggle", "select", "textarea"];
const CHARTS: [&str; 2] = ["line", "bar"];

#[derive(Clone, Copy)]
pub(super) struct Node<'v> {
    fields: &'v Map<String, Value>,
}

impl<'v> Node<'v> {
    pub(super) fn of(value: &'v Value, what: &str) -> Result<Self, String> {
        let fields = value
            .as_object()
            .ok_or_else(|| format!("{what} must be an object"))?;

        Ok(Self { fields })
    }

    pub(super) fn get(self, name: &str) -> Option<&'v Value> {
        self.fields.get(name)
    }

    pub(super) fn text(self, name: &str) -> Result<&'v str, String> {
        self.get(name)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("a view node needs {name} as text"))
    }

    pub(super) fn list(self, name: &str) -> Result<&'v Vec<Value>, String> {
        self.get(name)
            .and_then(Value::as_array)
            .ok_or_else(|| format!("a view node needs {name} as a list"))
    }

    pub(super) fn one_of(self, name: &str, allowed: &[&str], required: bool) -> Result<(), String> {
        match self.get(name) {
            None if !required => Ok(()),
            Some(Value::String(value)) if allowed.contains(&value.as_str()) => Ok(()),
            _ => Err(format!("{name} must be one of {}", allowed.join(", "))),
        }
    }

    pub(super) fn check_text(self) -> Result<(), String> {
        self.one_of("style", &STYLES, false)?;
        self.text("text")?;

        Ok(())
    }

    pub(super) fn check_log(self) -> Result<(), String> {
        let lines = self.list("lines")?;

        if lines.len() > MOST_LINES || !lines.iter().all(Value::is_string) {
            return Err(format!("a log holds at most {MOST_LINES} lines of text"));
        }

        Ok(())
    }

    pub(super) fn check_table(self) -> Result<(), String> {
        let columns = self.list("columns")?;
        if !columns.iter().all(Value::is_string) {
            return Err("table columns must be text".to_owned());
        }

        self.list("rows")?.iter().try_for_each(|row| {
            let cells = row
                .as_array()
                .ok_or("a table row must be a list of cells")?;

            if cells.len() != columns.len() || !cells.iter().all(is_scalar) {
                return Err("every table row holds one plain value per column".to_owned());
            }

            Ok(())
        })
    }

    pub(super) fn check_key_values(self) -> Result<(), String> {
        self.list("items")?.iter().try_for_each(|item| {
            let item = Self::of(item, "a key-value item")?;
            item.text("key")?;

            if item.get("value").is_some_and(is_scalar) {
                return Ok(());
            }

            Err("a key-value item needs a plain value".to_owned())
        })
    }

    pub(super) fn check_field(self) -> Result<(), String> {
        self.text("name")?;
        self.text("label")?;
        self.one_of("input", &INPUTS, true)?;

        let plain_options = |options: &Value| {
            options
                .as_array()
                .is_some_and(|options| options.iter().all(Value::is_string))
        };

        match self.get("options") {
            Some(options) if !plain_options(options) => {
                Err("field options must be a list of text".to_owned())
            }
            _ => Ok(()),
        }
    }

    pub(super) fn check_chart(self) -> Result<(), String> {
        self.one_of("kind", &CHARTS, true)?;

        self.list("series")?.iter().try_for_each(|series| {
            let series = Self::of(series, "a chart series")?;
            series.text("name")?;

            if series.list("points")?.iter().all(is_point) {
                return Ok(());
            }

            Err("chart points are [x, y] pairs of numbers".to_owned())
        })
    }
}

fn is_scalar(value: &Value) -> bool {
    matches!(
        value,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
    )
}

fn is_point(point: &Value) -> bool {
    point
        .as_array()
        .is_some_and(|pair| pair.len() == 2 && pair.iter().all(Value::is_number))
}
