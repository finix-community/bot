use eros::Context;
use octocrab::Octocrab;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub struct GithubClient {
    octocrab: Octocrab,
    org: String,
}

#[derive(Serialize)]
struct SearchCommitsParams<'a> {
    q: &'a str,
    per_page: u8,
}

#[derive(Deserialize)]
struct SearchCommitsResponse {
    items: Vec<CommitItem>,
}

#[derive(Deserialize)]
struct CommitItem {
    repository: RepoRef,
}

#[derive(Deserialize)]
struct RepoRef {
    name: String,
}

impl GithubClient {
    pub fn new(org: impl Into<String>) -> Self {
        Self {
            octocrab: Octocrab::default(),
            org: org.into(),
        }
    }

    pub async fn contributed_repos(&self, github_login: &str) -> eros::Result<BTreeSet<String>> {
        let query = format!("org:{} author:{}", self.org, github_login);
        let params = SearchCommitsParams {
            q: &query,
            per_page: 100,
        };

        let response: SearchCommitsResponse = self
            .octocrab
            .get("search/commits", Some(&params))
            .await
            .context("querying github search/commits")?;

        Ok(response
            .items
            .into_iter()
            .map(|item| item.repository.name)
            .collect())
    }
}
