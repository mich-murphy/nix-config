use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidId(pub &'static str);

impl fmt::Display for InvalidId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for InvalidId {}

macro_rules! text_id {
    ($name:ident, $check:expr, $message:literal) => {
        #[derive(
            Debug,
            Clone,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            Serialize,
            Deserialize,
            schemars::JsonSchema,
        )]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl FromStr for $name {
            type Err = InvalidId;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                if $check(value) {
                    Ok(Self(value.to_owned()))
                } else {
                    Err(InvalidId($message))
                }
            }
        }

        impl TryFrom<String> for $name {
            type Error = InvalidId;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                value.parse()
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

fn issue(value: &str) -> bool {
    value.split_once('-').is_some_and(|(project, number)| {
        !project.is_empty()
            && project
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
            && !number.is_empty()
            && number.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn slot(value: &str) -> bool {
    value
        .strip_prefix("worker")
        .is_some_and(|number| matches!(number.parse::<u8>(), Ok(1..=9)))
}

fn lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn model(value: &str) -> bool {
    value
        .split_once('/')
        .is_some_and(|(provider, id)| !provider.is_empty() && !id.is_empty())
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

text_id!(TaskId, issue, "invalid task id");
text_id!(IssueKey, issue, "invalid issue key");
text_id!(SlotId, slot, "invalid slot");
text_id!(Sha, |value| lower_hex(value, 40), "invalid SHA");
text_id!(Digest, |value| lower_hex(value, 64), "invalid digest");
text_id!(ModelId, model, "model must be provider/id");
text_id!(CriterionId, nonempty, "empty criterion id");
text_id!(FindingId, nonempty, "empty finding id");
text_id!(JiraStatus, nonempty, "empty Jira status");
text_id!(TransitionId, nonempty, "empty transition id");

impl From<TaskId> for IssueKey {
    fn from(value: TaskId) -> Self {
        Self(value.0)
    }
}

macro_rules! numeric_id {
    ($name:ident) => {
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            Serialize,
            Deserialize,
            schemars::JsonSchema,
        )]
        pub struct $name(pub u64);
    };
}

numeric_id!(AuthorityId);
numeric_id!(DeliveryId);
numeric_id!(LaunchId);
numeric_id!(OperationId);
numeric_id!(UseId);
numeric_id!(PrNumber);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_reject_invalid_text() {
        assert!("GAIN-847".parse::<TaskId>().is_ok());
        assert!("gain-1".parse::<TaskId>().is_err());
        assert!("worker9".parse::<SlotId>().is_ok());
        assert!("worker10".parse::<SlotId>().is_err());
        assert!("A".repeat(40).parse::<Sha>().is_err());
        assert!("openai/gpt-5".parse::<ModelId>().is_ok());
    }
}
