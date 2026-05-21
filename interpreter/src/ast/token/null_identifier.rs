#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NullIdentifier;

impl std::fmt::Display for NullIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "_")
    }
}
