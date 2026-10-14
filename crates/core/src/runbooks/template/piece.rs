use crate::runbooks::param::ParamName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Piece {
    Text(String),
    Param(ParamName),
}
