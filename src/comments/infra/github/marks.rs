//! The read marks on the pull request.

use serde::Serialize;

use super::*;
use crate::comments::domain::{PullFile, ReadMarks};

impl ReadMarks for GitHub {
    /// One request, however many files.
    ///
    /// The REST API has no word for this: whether a file is marked as read is
    /// per person, and only the GraphQL schema exposes it. So this is the one
    /// call that speaks the other language, and it aliases a mutation per path
    /// rather than sending a request each.
    ///
    /// The paths travel as variables and never inside the query text. A path
    /// with a quote in it would otherwise end the string and the rest would be
    /// read as GraphQL.
    fn mark_read(&self, pull: &str, paths: &[String]) -> Result<usize> {
        self.views(MARK, pull, paths)
    }

    fn mark_unread(&self, pull: &str, paths: &[String]) -> Result<usize> {
        self.views(UNMARK, pull, paths)
    }

    fn files(&self, pull: &str) -> Result<Vec<PullFile>> {
        let token = self.token()?;
        let mut all = Vec::new();
        let mut after: Option<String> = None;
        // Page by page: a pull request with more files than one page holds is
        // ordinary, and a file past the first page left out was one a sync
        // never unmarked.
        loop {
            let data = self.graphql_call(
                &token,
                VIEWED_FILES,
                serde_json::json!({ "pull": pull, "after": after }),
            )?;
            let files = data.pointer("/node/files");
            all.extend(
                files
                    .and_then(|f| f.pointer("/nodes"))
                    .and_then(|n| n.as_array())
                    .into_iter()
                    .flatten()
                    .filter_map(|f| {
                        Some(PullFile {
                            path: f.get("path")?.as_str()?.to_string(),
                            viewed: f.get("viewerViewedState").and_then(|s| s.as_str())
                                == Some("VIEWED"),
                        })
                    }),
            );
            match files.and_then(next_page) {
                Some(cursor) => after = Some(cursor),
                None => return Ok(all),
            }
        }
    }
}

impl GitHub {
    /// One mutation per path, marking or unmarking, a batch per request.
    ///
    /// Batches because GitHub runs about ninety mutations in one request and
    /// answers the rest with "Resource limits for this query exceeded": sent
    /// whole, a sync of two hundred files marked ninety and called it done.
    fn views(&self, mutation: &str, pull: &str, paths: &[String]) -> Result<usize> {
        let mut marked = 0;
        for batch in paths.chunks(BATCH) {
            marked += self.batch(mutation, pull, batch)?;
        }
        Ok(marked)
    }

    fn batch(&self, mutation: &str, pull: &str, paths: &[String]) -> Result<usize> {
        let token = self.token()?;
        let Mutations { query, variables } = Mutations::over(mutation, pull, paths);
        outcome(&self.graphql(&token, &query, variables)?)
    }
}

/// What one batch came to.
///
/// Nothing marked and the host saying why is a refusal, not an empty answer.
/// Reported as a count it would arrive on the screen as "there was nothing to
/// tick", which is what somebody whose token lacks the permission would be
/// told. A limit hit halfway is a refusal too: the files past it were never
/// asked about.
fn outcome(answer: &serde_json::Value) -> Result<usize> {
    match (took(answer), refusal(answer)) {
        (0, Some(why)) => Err(CommentError::Refused { what: why }.into()),
        (_, Some(why)) if why.contains("Resource limits") => {
            Err(CommentError::Refused { what: why }.into())
        }
        (marked, _) => Ok(marked),
    }
}

/// Mutations in one request. Well under the ninety or so GitHub runs before it
/// stops, so a batch never meets the limit.
const BATCH: usize = 50;

/// A mutation per path, aliased so one request carries them all.
///
/// Written out as text because a GraphQL document is text; what must never be
/// text is the data, which rides in `variables`.
#[derive(Serialize)]
pub(super) struct Mutations {
    query: String,
    variables: serde_json::Value,
}

impl Mutations {
    fn over(mutation: &str, pull: &str, paths: &[String]) -> Self {
        let args = (0..paths.len())
            .map(|i| format!("$p{i}: String!"))
            .collect::<Vec<_>>()
            .join(", ");
        let calls = (0..paths.len())
            .map(|i| {
                format!(
                    "m{i}: {mutation}(input: {{pullRequestId: $id, path: $p{i}}}) \
                     {{ clientMutationId }}"
                )
            })
            .collect::<Vec<_>>()
            .join(" ");

        let mut variables = serde_json::Map::new();
        variables.insert("id".into(), pull.into());
        for (i, path) in paths.iter().enumerate() {
            variables.insert(format!("p{i}"), path.as_str().into());
        }

        Self {
            query: format!("mutation($id: ID!, {args}) {{ {calls} }}"),
            variables: serde_json::Value::Object(variables),
        }
    }
}

/// How many of the aliases came back with something in them.
///
/// GraphQL answers 200 and puts what failed in `errors`, so the status says
/// nothing here. Counting the aliases that are not null is also what makes a
/// path the pull request does not know fail on its own without taking the rest
/// of the request with it.
pub(super) fn took(answer: &serde_json::Value) -> usize {
    answer
        .get("data")
        .and_then(|d| d.as_object())
        .map(|fields| fields.values().filter(|v| !v.is_null()).count())
        .unwrap_or(0)
}

pub(super) const MARK: &str = "markFileAsViewed";
pub(super) const UNMARK: &str = "unmarkFileAsViewed";
pub(super) const VIEWED_FILES: &str = "query($pull: ID!, $after: String) { node(id: $pull) { ... on PullRequest { files(first: 100, after: $after) { nodes { path viewerViewedState } pageInfo { hasNextPage endCursor } } } } }";

/// Where the next page starts, when there is one.
fn next_page(connection: &serde_json::Value) -> Option<String> {
    let page = connection.get("pageInfo")?;
    if page.get("hasNextPage")?.as_bool()? {
        Some(page.get("endCursor")?.as_str()?.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_request_carries_a_mutation_per_file() {
        // Thirty files ticked is one round trip, not thirty. The paths ride in
        // `variables`, so a path with a quote in it cannot end the query string
        // and have the rest read as GraphQL.
        let body = serde_json::to_value(Mutations::over(
            MARK,
            "PR_kwDO",
            &["src/a.rs".into(), "he said \"hi\".rs".into()],
        ))
        .unwrap();

        let query = body["query"].as_str().unwrap();
        assert!(query.contains("m0: markFileAsViewed"), "{query}");
        assert!(query.contains("m1: markFileAsViewed"), "{query}");
        assert!(
            query.contains("$id: ID!, $p0: String!, $p1: String!"),
            "{query}"
        );
        assert!(
            !query.contains("src/a.rs"),
            "the data belongs in the variables"
        );

        assert_eq!(body["variables"]["id"], "PR_kwDO");
        assert_eq!(body["variables"]["p0"], "src/a.rs");
        assert_eq!(body["variables"]["p1"], "he said \"hi\".rs");
    }

    #[test]
    fn a_path_the_pull_request_does_not_know_fails_on_its_own() {
        // GraphQL answers 200 and puts what went wrong in `errors`, so the
        // status says nothing. What counts is how many aliases came back with
        // something in them.
        let answered = serde_json::json!({
            "data": { "m0": { "clientMutationId": null }, "m1": null },
            "errors": [{ "message": "Could not resolve to a file", "path": ["m1"] }],
        });

        assert_eq!(took(&answered), 1);
    }

    #[test]
    fn a_request_the_host_refused_outright_is_a_failure_and_not_a_zero() {
        // The shape a token without the permission comes back in: nothing
        // marked, and the reason only in `errors`. Counted, it would reach the
        // screen as "there was nothing to tick".
        let refused = serde_json::json!({
            "data": null,
            "errors": [{ "message": "Resource not accessible by personal access token" }],
        });

        assert_eq!(took(&refused), 0);
        assert_eq!(
            refusal(&refused).unwrap(),
            "Resource not accessible by personal access token"
        );

        assert!(refusal(&serde_json::json!({ "data": { "m0": {} } })).is_none());
    }

    #[test]
    fn the_viewed_files_are_read_to_the_last_page() {
        // One page holds a hundred files, which a pull request easily passes.
        let more = serde_json::json!({ "pageInfo": { "hasNextPage": true, "endCursor": "c100" } });
        let last = serde_json::json!({ "pageInfo": { "hasNextPage": false, "endCursor": "c163" } });

        assert_eq!(next_page(&more).as_deref(), Some("c100"));
        assert_eq!(next_page(&last), None);
    }

    #[test]
    fn a_batch_cut_short_by_the_hosts_limit_is_refused_and_not_counted() {
        // GitHub runs about ninety and answers the rest with this; counted, it
        // reached the screen as a sync that worked.
        let cut = serde_json::json!({
            "data": { "m0": { "clientMutationId": null }, "m1": null },
            "errors": [{ "message": "Resource limits for this query exceeded." }],
        });
        let unknown = serde_json::json!({
            "data": { "m0": { "clientMutationId": null }, "m1": null },
            "errors": [{ "message": "Could not resolve to a file", "path": ["m1"] }],
        });

        assert!(outcome(&cut).is_err());
        assert_eq!(
            outcome(&unknown).unwrap(),
            1,
            "a path the pull request lacks fails alone"
        );
    }
}
