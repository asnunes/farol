//! Finding the pull request, and opening one.

use super::*;
use crate::comments::domain::{OpenedPull, PullDraft, PullRequests, Readiness};

impl PullRequests for GitHub {
    fn readiness(&self, branch: &str, head: &str) -> Result<Readiness> {
        let Some(token) = self.credentials.token(&self.remote.host)? else {
            return Ok(Readiness::NoToken);
        };

        // By the commit first: it finds the pull request of a fork, and of a
        // branch checked out locally under a name of its own, where the branch
        // name would find nothing. A commit the host has never seen answers
        // 422, which is not a refusal — it is the next lookup's turn.
        let by_commit = self
            .http
            .get(&token, &format!("{}/commits/{}/pulls", self.repo(), head))?;
        if by_commit.refused() {
            return Ok(Readiness::TokenRefused);
        }
        let open: Vec<Pull> = match by_commit.ok() {
            true => pulls(by_commit.json()?)
                .into_iter()
                .filter(|pull| pull.open)
                .collect(),
            false => vec![],
        };
        let showing: Vec<&Pull> = open.iter().filter(|pull| pull.head == head).collect();
        let found = match (showing.as_slice(), open.as_slice()) {
            ([one], _) => Some((*one).clone()),
            // It holds this commit and has moved on since: the head check
            // downstream says which way to move.
            ([], [one]) => Some(one.clone()),
            ([], []) => None,
            _ => {
                return Ok(Readiness::Ambiguous {
                    numbers: open.iter().map(|pull| pull.number).collect(),
                });
            }
        };

        // Then by the branch, which is what still finds the pull request when
        // the local commits are not pushed yet.
        let found = match found {
            Some(pull) => Some(pull),
            None => {
                let by_branch = self.http.get(
                    &token,
                    &format!(
                        "{}/pulls?state=open&head={}:{}",
                        self.repo(),
                        self.owner(),
                        encoded(branch)
                    ),
                )?;
                if by_branch.refused() {
                    return Ok(Readiness::TokenRefused);
                }
                by_branch
                    .json()?
                    .and_then(|body| pulls(Some(body)).into_iter().next())
            }
        };

        if let Some(pull) = found {
            // One more call, and only here: the screen has to know whose pull
            // request this is before it offers a verdict, and the answer is
            // the same for every state that is not Ready.
            let me = self.logins.get_or(&token, || {
                let me = self.read(self.http.get(&token, &format!("{}/user", self.api()))?)?;
                Ok(me
                    .get("login")
                    .and_then(|l| l.as_str())
                    .unwrap_or_default()
                    .to_string())
            })?;
            return Ok(Readiness::Ready {
                mine: !me.is_empty() && me == pull.author,
                pull_request: pull.number,
                id: pull.id,
                head: pull.head,
                author: pull.author,
                url: pull.url,
            });
        }

        // No pull request, and two very different reasons for it. Asking the
        // host whether the branch is there beats reading `refs/remotes`, which
        // is only as fresh as the last fetch.
        let pushed = self.http.get(
            &token,
            &format!("{}/branches/{}", self.repo(), encoded(branch)),
        )?;
        if pushed.refused() {
            return Ok(Readiness::TokenRefused);
        }
        Ok(match pushed.ok() {
            false => Readiness::BranchNotPushed,
            true => Readiness::NoPullRequest {
                open_at: format!(
                    "https://{}/{}/compare/{}?expand=1",
                    self.remote.host,
                    self.remote.slug,
                    encoded(branch)
                ),
            },
        })
    }

    fn open_pull(&self, base: &str, branch: &str, draft: &PullDraft) -> Result<OpenedPull> {
        let token = self.token()?;
        let body = serde_json::json!({
            "title": draft.title,
            "body": draft.body,
            "head": branch,
            "base": base,
        });
        let opened = self.read(self.http.post(
            &token,
            &format!("{}/pulls", self.repo()),
            body.to_string(),
        )?)?;
        let number = opened.get("number").and_then(|n| n.as_u64());
        let url = opened.get("html_url").and_then(|u| u.as_str());
        match (number, url) {
            (Some(number), Some(url)) => Ok(OpenedPull {
                number: number as u32,
                url: url.to_string(),
            }),
            _ => Err(refused("the host opened no pull request")),
        }
    }
}

/// The pull requests a listing names, as farol needs them. Both listings —
/// by commit and by branch — answer in this shape.
pub(super) fn pulls(body: Option<serde_json::Value>) -> Vec<Pull> {
    let Some(serde_json::Value::Array(all)) = body else {
        return vec![];
    };
    all.iter()
        .filter_map(|pull| {
            Some(Pull {
                number: pull.get("number")?.as_u64()? as u32,
                id: pull.get("node_id")?.as_str()?.to_string(),
                head: pull.get("head")?.get("sha")?.as_str()?.to_string(),
                author: pull.get("user")?.get("login")?.as_str()?.to_string(),
                url: pull
                    .get("html_url")
                    .and_then(|u| u.as_str())
                    .unwrap_or_default()
                    .to_string(),
                open: pull.get("state").and_then(|s| s.as_str()).unwrap_or("open") == "open",
            })
        })
        .collect()
}

/// What the listing says about one pull request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Pull {
    number: u32,
    /// The host's own name for it, which GraphQL addresses it by.
    id: String,
    /// The commit it is showing, which every comment is anchored against.
    head: String,
    /// Who opened it, which decides whether approving is even on the table.
    author: String,
    /// Its page.
    url: String,
    /// The listing by commit includes merged and closed ones.
    open: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_open_pull_request_is_read_with_the_commit_it_is_showing() {
        // The commit is the whole reason to ask: comments anchor to line
        // numbers, and those only mean anything against one commit. The node id
        // comes along because GraphQL addresses the pull request by it, and the
        // author because approving your own pull request is not a thing GitHub
        // allows.
        let body = serde_json::json!([
            {
                "number": 12,
                "node_id": "PR_kwDO",
                "head": {"sha": "abc1234"},
                "user": {"login": "asnunes"},
                "state": "open",
                "html_url": "https://github.com/asnunes/farol/pull/12",
            },
            {
                "number": 3,
                "node_id": "PR_old",
                "head": {"sha": "abc1234"},
                "user": {"login": "asnunes"},
                "state": "closed",
            },
        ]);

        let all = pulls(Some(body));
        assert_eq!(
            all[0],
            Pull {
                number: 12,
                id: "PR_kwDO".into(),
                head: "abc1234".into(),
                author: "asnunes".into(),
                url: "https://github.com/asnunes/farol/pull/12".into(),
                open: true,
            }
        );
        assert!(
            !all[1].open,
            "a merged or closed one is listed by commit too"
        );
        assert!(pulls(Some(serde_json::json!([]))).is_empty());
    }
}
