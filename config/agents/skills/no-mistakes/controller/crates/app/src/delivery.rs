use domain::{command::PublishStep, ports::MergeMethod};
use std::path::PathBuf;

pub(super) struct PublishInput {
    pub step: PublishStep,
    pub title: Option<String>,
    pub body: Option<PathBuf>,
    pub method: Option<MergeMethod>,
}
