pub mod authentication_actor;
pub mod authentication_token_store_actor;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Token(pub String);

impl Token {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().into())
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Username(String);

#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
struct Deadline(std::time::Instant);
