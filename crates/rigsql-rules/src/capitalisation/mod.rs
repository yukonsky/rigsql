pub mod cp01;
pub mod cp02;
pub mod cp03;
pub mod cp04;
pub mod cp05;

/// Shared capitalisation policy for CP01, CP02, CP03, CP05.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapitalisationPolicy {
    Upper,
    Lower,
    Capitalise,
    Consistent,
}

impl CapitalisationPolicy {
    /// Parse a capitalisation_policy setting string.
    pub fn from_config(s: &str) -> Self {
        match s {
            "lower" => Self::Lower,
            "capitalise" | "capitalize" => Self::Capitalise,
            "consistent" => Self::Consistent,
            _ => Self::Upper,
        }
    }
}
