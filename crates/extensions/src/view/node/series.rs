use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(in crate::view) struct Series {
    #[serde(rename = "name")]
    _name: String,

    #[serde(rename = "points")]
    _points: Vec<[f64; 2]>,
}
