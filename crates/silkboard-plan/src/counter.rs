//! Exact positive counters, encoded as strings to preserve browser precision.
use crate::InvalidPlan;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

/// A positive signed-64-bit counter with canonical decimal-string JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Counter(i64);
impl Counter {
    /// Construct a positive counter.
    ///
    /// # Errors
    /// Returns invalid input for zero or a negative value.
    pub const fn new(value: i64) -> Result<Self, InvalidPlan> {
        if value > 0 {
            Ok(Self(value))
        } else {
            Err(InvalidPlan)
        }
    }
    /// Exact integer value suitable for `SQLite` ordering.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}
impl std::fmt::Display for Counter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl std::str::FromStr for Counter {
    type Err = InvalidPlan;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || value.len() > 19
            || value.starts_with('0')
            || !value.bytes().all(|c| c.is_ascii_digit())
        {
            return Err(InvalidPlan);
        }
        Self::new(value.parse().map_err(|_| InvalidPlan)?)
    }
}
impl Serialize for Counter {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
impl<'de> Deserialize<'de> for Counter {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(D::Error::custom)
    }
}
