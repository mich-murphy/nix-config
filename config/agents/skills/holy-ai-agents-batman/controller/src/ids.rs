//! Identifiers parsed once at the edge. An invalid value is rejected during
//! deserialisation, so the interior never re-validates.
use serde::{Deserialize, Serialize};
use std::{borrow::Borrow, fmt, ops::Deref, str::FromStr};

macro_rules! identifier {
    ($name:ident, $valid:expr, $message:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl FromStr for $name {
            type Err = anyhow::Error;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                anyhow::ensure!($valid(value), "{}: {value:?}", $message);
                Ok(Self(value.to_owned()))
            }
        }

        impl TryFrom<String> for $name {
            type Error = anyhow::Error;

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

        impl Borrow<str> for $name {
            fn borrow(&self) -> &str {
                &self.0
            }
        }

        impl Deref for $name {
            type Target = str;

            fn deref(&self) -> &str {
                &self.0
            }
        }

        impl PartialEq<str> for $name {
            fn eq(&self, other: &str) -> bool {
                self.0 == other
            }
        }

        impl PartialEq<String> for $name {
            fn eq(&self, other: &String) -> bool {
                self.0 == *other
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

fn task_id(value: &str) -> bool {
    let Some((prefix, number)) = value.split_once('-') else {
        return false;
    };
    !prefix.is_empty()
        && prefix
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        && !number.is_empty()
        && number.bytes().all(|b| b.is_ascii_digit())
}

fn slot_id(value: &str) -> bool {
    value.len() == 7
        && value.starts_with("worker")
        && matches!(value.as_bytes().get(6), Some(b'1'..=b'9'))
}

fn lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}

identifier!(TaskId, task_id, "invalid task id");
identifier!(SlotId, slot_id, "invalid slot; worker1 through worker9");
identifier!(Sha, |v| lower_hex(v, 40), "invalid commit SHA");
identifier!(Digest, |v| lower_hex(v, 64), "invalid digest");

impl Default for Digest {
    fn default() -> Self {
        Self("0".repeat(64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_id_requires_upper_prefix_and_number() {
        assert!("GAIN-847".parse::<TaskId>().is_ok());
        assert!("gain-1".parse::<TaskId>().is_err());
        assert!("GAIN-".parse::<TaskId>().is_err());
        assert!("GAIN".parse::<TaskId>().is_err());
    }

    #[test]
    fn slot_id_is_worker_one_to_nine() {
        assert!("worker1".parse::<SlotId>().is_ok());
        assert!("worker9".parse::<SlotId>().is_ok());
        assert!("worker0".parse::<SlotId>().is_err());
        assert!("worker10".parse::<SlotId>().is_err());
    }

    #[test]
    fn hex_ids_require_exact_lowercase_length() {
        assert!("a".repeat(40).parse::<Sha>().is_ok());
        assert!("A".repeat(40).parse::<Sha>().is_err());
        assert!("a".repeat(39).parse::<Sha>().is_err());
        assert!("0".repeat(64).parse::<Digest>().is_ok());
        assert!("0".repeat(63).parse::<Digest>().is_err());
    }

    #[test]
    fn wire_shape_is_a_plain_string() -> anyhow::Result<()> {
        let id: TaskId = serde_json::from_str("\"TASK-12\"")?;
        assert_eq!(serde_json::to_string(&id)?, "\"TASK-12\"");
        assert!(serde_json::from_str::<SlotId>("\"worker0\"").is_err());
        Ok(())
    }
}
