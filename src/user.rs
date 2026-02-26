use std::{
    fmt::{Display, Write},
    hash::{self, Hasher},
    num::ParseIntError,
    str::FromStr,
};

use snowflaked::Snowflake;

/// Concrete type for user ID's.
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct UserId(pub u64);

impl hash::Hash for UserId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0);
    }
}

impl Snowflake for UserId {
    fn from_parts(timestamp: u64, instance: u64, sequence: u64) -> Self {
        Self(u64::from_parts(timestamp, instance, sequence))
    }

    fn timestamp(&self) -> u64 {
        self.0.timestamp()
    }

    fn instance(&self) -> u64 {
        self.0.instance()
    }

    fn sequence(&self) -> u64 {
        self.0.sequence()
    }
}

impl Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.to_string().as_str())
    }
}

/// Allows for calling parse() on strings to convert them to a user ID.
impl FromStr for UserId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(UserId(u64::from_str(s)?))
    }
}
