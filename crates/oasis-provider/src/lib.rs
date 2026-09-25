//! Provider trait and mock LLM provider for Oasis.

mod mock;
mod traits;

pub use mock::{MockProvider, MockScript};
pub use traits::{Provider, ProviderRequest, ProviderResponse};
