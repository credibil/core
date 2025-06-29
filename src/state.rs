//! # State

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::datastore::Datastore;

const STATE: &str = "STATE";

/// The `StateStore` trait is implemented to provide concrete storage and
/// retrieval of retrieve server state between requests.
pub trait StateStore: Send + Sync {
    /// Store state using the provided key. The expiry parameter indicates
    /// when data can be expunged from the state store.
    fn put<T: Serialize + Sync>(
        &self, owner: &str, key: &str, state: &State<T>,
    ) -> impl Future<Output = Result<()>> + Send;

    /// Retrieve data using the provided key.
    fn get<T: DeserializeOwned>(
        &self, owner: &str, key: &str,
    ) -> impl Future<Output = Result<State<T>>> + Send;

    /// Remove data using the key provided.
    fn purge(&self, owner: &str, key: &str) -> impl Future<Output = Result<()>> + Send;
}

/// State is used to persist request information between issuance steps in the
/// Credential issuance process.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct State<T> {
    /// Body holds data relevant to the current state.
    pub body: T,

    /// Time state should expire.
    pub expires_at: DateTime<Utc>,
}

impl<T> State<T> {
    /// Determines whether state has expired or not.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.expires_at.signed_duration_since(Utc::now()).num_seconds() < 0
    }
}

impl<T: Serialize> From<T> for State<T> {
    fn from(body: T) -> Self {
        Self {
            body,
            expires_at: Utc::now(), //+ Expire::Authorized.duration(),
        }
    }
}

impl<B> StateStore for B
where
    B: Datastore,
{
    #[allow(unused)]
    async fn put<T: Serialize + Sync>(
        &self, owner: &str, key: &str, state: &State<T>,
    ) -> Result<()> {
        let state = serde_json::to_vec(state)?;
        Datastore::delete(self, owner, STATE, key).await?;
        Datastore::put(self, owner, STATE, key, &state).await
    }

    async fn get<T: DeserializeOwned>(&self, owner: &str, key: &str) -> Result<State<T>> {
        let Some(data) = Datastore::get(self, owner, STATE, key).await? else {
            return Err(anyhow!("no matching item in state store"));
        };
        Ok(serde_json::from_slice(&data)?)
    }

    async fn purge(&self, owner: &str, key: &str) -> Result<()> {
        Datastore::delete(self, owner, STATE, key).await
    }
}
