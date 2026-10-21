//! dialogue-maker on GitHub: signing in (OAuth device flow), the repository a project's
//! `origin` points at, pushing a branch, opening its pull request. The only part of
//! dialogue-maker that uses the network, and only when the writer asks it to.

use std::fmt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// dialogue-maker's OAuth app on GitHub. Not a secret: the device flow has none.
pub const CLIENT_ID: &str = "Ov23li9jPpUeZBeyVUIY";

/// The permission asked for: push to the team's repositories, private ones too, and open
/// pull requests on them.
pub const SCOPE: &str = "repo";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubError(pub String);

impl fmt::Display for GitHubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for GitHubError {}

impl From<ureq::Error> for GitHubError {
    fn from(e: ureq::Error) -> Self {
        Self(format!("cannot reach GitHub: {e}"))
    }
}

impl From<git2::Error> for GitHubError {
    fn from(e: git2::Error) -> Self {
        Self(e.message().to_owned())
    }
}

fn fail<T>(message: impl Into<String>) -> Result<T, GitHubError> {
    Err(GitHubError(message.into()))
}

/// Where GitHub is. Tests point this at a fake one.
#[derive(Debug, Clone)]
pub struct GitHub {
    /// The site: `https://github.com`.
    pub web: String,
    /// The API: `https://api.github.com`.
    pub api: String,
    /// Where repositories are pushed to, as `{git}/{owner}/{name}.git`: the site.
    pub git: String,
    agent: ureq::Agent,
}

impl Default for GitHub {
    fn default() -> Self {
        Self::new("https://github.com", "https://api.github.com")
    }
}

/// A sign-in under way: the code the writer confirms on GitHub, and where.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct SignIn {
    pub user_code: String,
    pub verification_uri: String,
    /// Seconds until the code expires.
    #[ts(type = "number")]
    pub expires_in: u64,
    /// Seconds to wait between polls.
    #[ts(type = "number")]
    pub interval: u64,
    #[serde(skip)]
    device_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct Account {
    pub login: String,
    pub name: Option<String>,
}

/// A repository on GitHub, as `owner/name`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Repo {
    pub owner: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct PullRequest {
    #[ts(type = "number")]
    pub number: u64,
    pub url: String,
    /// False when one was already open for the branch.
    pub created: bool,
}

impl GitHub {
    pub fn new(web: &str, api: &str) -> Self {
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(30)))
            .user_agent("dialogue-maker")
            .build();
        Self {
            web: web.trim_end_matches('/').to_owned(),
            api: api.trim_end_matches('/').to_owned(),
            git: web.trim_end_matches('/').to_owned(),
            agent: ureq::Agent::new_with_config(config),
        }
    }

    /// Ask GitHub for a code the writer confirms in the browser.
    pub fn start_sign_in(&self) -> Result<SignIn, GitHubError> {
        #[derive(Deserialize)]
        struct Code {
            device_code: String,
            user_code: String,
            verification_uri: String,
            expires_in: u64,
            interval: u64,
        }
        let mut response = self
            .agent
            .post(format!("{}/login/device/code", self.web))
            .header("Accept", "application/json")
            .send_form([("client_id", CLIENT_ID), ("scope", SCOPE)])?;
        if !response.status().is_success() {
            return fail(format!(
                "GitHub would not start a sign-in ({})",
                response.status()
            ));
        }
        let code: Code = response.body_mut().read_json()?;
        Ok(SignIn {
            user_code: code.user_code,
            verification_uri: code.verification_uri,
            expires_in: code.expires_in,
            interval: code.interval.max(1),
            device_code: code.device_code,
        })
    }

    /// Wait until the writer confirms the code (or refuses, or it expires, or `cancel` is
    /// set); returns the token.
    pub fn finish_sign_in(
        &self,
        sign_in: &SignIn,
        cancel: &AtomicBool,
    ) -> Result<String, GitHubError> {
        #[derive(Deserialize)]
        struct Answer {
            access_token: Option<String>,
            error: Option<String>,
            interval: Option<u64>,
        }
        let deadline = Instant::now() + Duration::from_secs(sign_in.expires_in);
        let mut interval = Duration::from_secs(sign_in.interval);
        loop {
            // Wait first, in small steps so that a cancel is quick.
            let until = Instant::now() + interval;
            while Instant::now() < until {
                if cancel.load(Ordering::Relaxed) {
                    return fail("sign-in cancelled");
                }
                std::thread::sleep(Duration::from_millis(100).min(interval));
            }
            if Instant::now() > deadline {
                return fail("the code expired before it was confirmed; start again");
            }
            let mut response = self
                .agent
                .post(format!("{}/login/oauth/access_token", self.web))
                .header("Accept", "application/json")
                .send_form([
                    ("client_id", CLIENT_ID),
                    ("device_code", sign_in.device_code.as_str()),
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ])?;
            let answer: Answer = response.body_mut().read_json()?;
            if let Some(token) = answer.access_token {
                return Ok(token);
            }
            match answer.error.as_deref() {
                Some("authorization_pending") => {}
                Some("slow_down") => {
                    interval =
                        Duration::from_secs(answer.interval.unwrap_or(interval.as_secs() + 5))
                }
                Some("expired_token") => {
                    return fail("the code expired before it was confirmed; start again");
                }
                Some("access_denied") => return fail("the sign-in was refused on GitHub"),
                Some(other) => return fail(format!("GitHub refused the sign-in: {other}")),
                None => return fail("GitHub answered the sign-in with nothing"),
            }
        }
    }

    fn get(
        &self,
        token: &str,
        path: &str,
    ) -> Result<ureq::http::Response<ureq::Body>, GitHubError> {
        Ok(self
            .api_request(self.agent.get(format!("{}{path}", self.api)), token)
            .call()?)
    }

    fn api_request<B>(
        &self,
        request: ureq::RequestBuilder<B>,
        token: &str,
    ) -> ureq::RequestBuilder<B> {
        request
            .header("Accept", "application/vnd.github+json")
            .header("Authorization", format!("Bearer {token}"))
            .header("X-GitHub-Api-Version", "2022-11-28")
    }

    /// Who a token signs in as; an error when it no longer works.
    pub fn account(&self, token: &str) -> Result<Account, GitHubError> {
        let mut response = self.get(token, "/user")?;
        if response.status() == 401 {
            return fail("the GitHub sign-in no longer works; sign in again");
        }
        if !response.status().is_success() {
            return fail(format!(
                "GitHub did not say who you are ({})",
                response.status()
            ));
        }
        Ok(response.body_mut().read_json()?)
    }

    /// The branch pull requests go into by default.
    pub fn default_branch(&self, token: &str, repo: &Repo) -> Result<String, GitHubError> {
        #[derive(Deserialize)]
        struct Info {
            default_branch: String,
        }
        let mut response = self.get(token, &format!("/repos/{}/{}", repo.owner, repo.name))?;
        if response.status() == 404 {
            return fail(format!(
                "{}/{} is not on GitHub, or this account cannot see it",
                repo.owner, repo.name
            ));
        }
        if !response.status().is_success() {
            return fail(format!(
                "GitHub would not describe {}/{} ({})",
                repo.owner,
                repo.name,
                response.status()
            ));
        }
        Ok(response.body_mut().read_json::<Info>()?.default_branch)
    }

    /// Open a pull request from `branch` into `base`, or find the one already open.
    pub fn pull_request(
        &self,
        token: &str,
        repo: &Repo,
        branch: &str,
        base: &str,
        title: &str,
        body: &str,
    ) -> Result<PullRequest, GitHubError> {
        #[derive(Deserialize)]
        struct Found {
            number: u64,
            html_url: String,
        }
        let path = format!("/repos/{}/{}/pulls", repo.owner, repo.name);
        let mut open = self.get(
            token,
            &format!("{path}?state=open&head={}:{branch}", repo.owner),
        )?;
        if open.status().is_success()
            && let Some(found) = open
                .body_mut()
                .read_json::<Vec<Found>>()?
                .into_iter()
                .next()
        {
            return Ok(PullRequest {
                number: found.number,
                url: found.html_url,
                created: false,
            });
        }
        let mut response = self
            .api_request(self.agent.post(format!("{}{path}", self.api)), token)
            .send_json(
                serde_json::json!({ "title": title, "head": branch, "base": base, "body": body }),
            )?;
        if response.status().is_success() {
            let found: Found = response.body_mut().read_json()?;
            return Ok(PullRequest {
                number: found.number,
                url: found.html_url,
                created: true,
            });
        }
        // GitHub says why in `errors[].message` or `message`.
        let text = response.body_mut().read_to_string().unwrap_or_default();
        let detail: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
        let reason = detail["errors"][0]["message"]
            .as_str()
            .or(detail["message"].as_str())
            .unwrap_or("no reason given");
        fail(format!("GitHub would not open the pull request: {reason}"))
    }
}

/// The GitHub repository a remote URL points at (`https://github.com/o/r.git`,
/// `git@github.com:o/r.git`, `ssh://git@github.com/o/r`).
pub fn github_repo(url: &str) -> Option<Repo> {
    let rest = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))
        .or_else(|| url.strip_prefix("git@github.com:"))
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))?;
    let rest = rest.trim_end_matches('/').trim_end_matches(".git");
    let (owner, name) = rest.split_once('/')?;
    (!owner.is_empty() && !name.is_empty() && !name.contains('/')).then(|| Repo {
        owner: owner.to_owned(),
        name: name.to_owned(),
    })
}

/// The project's `origin`: its URL, and the GitHub repository when it is one.
pub fn origin(root: &Path) -> Result<Option<(String, Option<Repo>)>, GitHubError> {
    let repo = match git2::Repository::discover(root) {
        Ok(repo) => repo,
        Err(_) => return Ok(None),
    };
    let Ok(remote) = repo.find_remote("origin") else {
        return Ok(None);
    };
    let url = remote.url().ok().unwrap_or_default().to_owned();
    let github = github_repo(&url);
    Ok(Some((url, github)))
}

impl GitHub {
    /// Push a branch to `origin` (over HTTPS with the token when it is on GitHub, whatever
    /// address `origin` has). Only a fast-forward: never over commits on GitHub the writer
    /// does not have.
    pub fn push(&self, root: &Path, branch: &str, token: &str) -> Result<(), GitHubError> {
        let repo = git2::Repository::discover(root)?;
        let remote = repo
            .find_remote("origin")
            .map_err(|_| GitHubError("the repository has no `origin` to push to".into()))?;
        let url = remote.url().ok().unwrap_or_default().to_owned();
        let on_github = github_repo(&url);
        let push_url = match &on_github {
            Some(r) => format!("{}/{}/{}.git", self.git, r.owner, r.name),
            None => url,
        };
        let mut target = repo.remote_anonymous(&push_url)?;
        let mut refused: Option<String> = None;
        let mut asked = false;
        {
            let mut callbacks = git2::RemoteCallbacks::new();
            // The token goes to GitHub only, and once: a refused token is not offered again.
            callbacks.credentials(|_, _, _| {
                if on_github.is_none() {
                    return Err(git2::Error::from_str(
                        "`origin` asks for a password: push with git",
                    ));
                }
                if std::mem::replace(&mut asked, true) {
                    return Err(git2::Error::from_str(
                        "GitHub did not accept the sign-in: sign in again",
                    ));
                }
                git2::Cred::userpass_plaintext("x-access-token", token)
            });
            callbacks.push_update_reference(|_, status| {
                if let Some(status) = status {
                    refused = Some(status.to_owned());
                }
                Ok(())
            });
            let mut options = git2::PushOptions::new();
            options.remote_callbacks(callbacks);
            let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
            target.push(&[refspec.as_str()], Some(&mut options)).map_err(|e| {
            if e.code() == git2::ErrorCode::NotFastForward || e.message().contains("non-fastforward") {
                GitHubError(format!("the branch {branch} on GitHub has commits you do not have; bring them in with git first"))
            } else {
                GitHubError(format!("cannot push {branch}: {}", e.message()))
            }
        })?;
        }
        if let Some(reason) = refused {
            return fail(format!("GitHub refused {branch}: {reason}"));
        }
        // Remember what origin has now, as `git push` would.
        let head = repo
            .find_reference(&format!("refs/heads/{branch}"))?
            .peel_to_commit()?
            .id();
        repo.reference(
            &format!("refs/remotes/origin/{branch}"),
            head,
            true,
            "dialogue-maker: push",
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_urls() {
        let r = |owner: &str, name: &str| {
            Some(Repo {
                owner: owner.into(),
                name: name.into(),
            })
        };
        assert_eq!(
            github_repo("https://github.com/murparreira/easyscript.git"),
            r("murparreira", "easyscript")
        );
        assert_eq!(
            github_repo("git@github.com:BusyBoys/na-cabeca-do-bicho.git"),
            r("BusyBoys", "na-cabeca-do-bicho")
        );
        assert_eq!(github_repo("ssh://git@github.com/a/b"), r("a", "b"));
        assert_eq!(github_repo("https://gitlab.com/a/b.git"), None);
        assert_eq!(github_repo("https://github.com/a"), None);
    }
}
