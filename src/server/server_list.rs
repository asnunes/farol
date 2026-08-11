//! The running servers, for the terminal.

use std::fmt::{self, Display};

use super::registry::ServerEntry;

pub struct ServerList<'a>(pub &'a [ServerEntry]);

impl Display for ServerList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return writeln!(f, "No farol server is running.");
        }

        // The review each one is showing is the column being compared, so it is
        // padded to line up; the paths after it are read one at a time.
        let width = self.0.iter().map(|e| review(e).len()).max().unwrap_or(0);

        for entry in self.0 {
            writeln!(
                f,
                "{:>5}  {:<width$}  {}",
                entry.port,
                review(entry),
                entry.repo.display()
            )?;
        }
        Ok(())
    }
}

fn review(entry: &ServerEntry) -> String {
    format!("{} → {}", entry.branch, entry.base)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn entry(port: u16, branch: &str) -> ServerEntry {
        ServerEntry {
            port,
            pid: 1,
            repo: PathBuf::from("/w/farol"),
            branch: branch.into(),
            base: "main".into(),
        }
    }

    #[test]
    fn each_server_says_where_it_is_and_what_it_is_showing() {
        let out = ServerList(&[entry(4600, "feat/x")]).to_string();

        assert!(out.contains("4600"), "{out}");
        assert!(out.contains("feat/x → main"), "{out}");
        assert!(out.contains("/w/farol"), "{out}");
    }

    #[test]
    fn the_reviews_line_up_so_the_list_can_be_scanned() {
        let out =
            ServerList(&[entry(4600, "feat/a-very-long-branch"), entry(4601, "x")]).to_string();

        let repo_at: Vec<_> = out.lines().map(|l| l.find("/w/farol").unwrap()).collect();
        assert_eq!(repo_at[0], repo_at[1], "{out}");
    }

    #[test]
    fn nothing_running_says_so_rather_than_printing_emptiness() {
        // A blank response to `farol servers` reads as a broken command.
        assert_eq!(
            ServerList(&[]).to_string().trim(),
            "No farol server is running."
        );
    }
}
