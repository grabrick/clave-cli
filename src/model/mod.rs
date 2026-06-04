use crate::prelude::*;

pub(crate) mod commands;
pub(crate) mod constants;
pub(crate) mod effort;
pub(crate) mod language;
pub(crate) mod mode;

pub(crate) use commands::*;
pub(crate) use constants::*;
pub(crate) use effort::*;
pub(crate) use language::*;
pub(crate) use mode::*;

pub(crate) type AnyResult<T> = Result<T, Box<dyn Error>>;
