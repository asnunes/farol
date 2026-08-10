//! The HTTP surface, against a real server.
//!
//! Driven over the wire rather than by handing the router a request: the
//! browser talks to a process, and everything between the socket and the map on
//! disk — the composition root, the listener, git, the store — is part of what
//! can break. Injecting fakes underneath would test the handlers and vouch for
//! none of that.

mod common;

use common::{BIN, Repo, numbered};
use serde_json::Value;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// A running `farol serve`, killed when the test ends.
struct Serving {
    repo: Repo,
    child: Child,
    port: u16,
}

impl Serving {
    /// A branch with one mapped file and a note on it, already being served.
    fn new() -> Self {
        Self::with(&["--no-watch"])
    }

    fn with(extra: &[&str]) -> Self {
        let repo = Repo::new();
        repo.feature();
        repo.write("src/b.rs", &numbered(20));
        repo.write("Cargo.lock", "checksums\n");
        repo.commit("more to review");
        repo.derive();
        repo.core_block(&["src/a.rs"]);
        repo.ok(&["file", "update", "core", "src/a.rs", "--note", "start here"]);
        repo.ok(&[
            "line",
            "add",
            "core",
            "src/a.rs",
            "10-12",
            "--note",
            "the actual fix",
        ]);
        repo.ok(&[
            "block",
            "add",
            "wiring",
            "--title",
            "The wiring",
            "--context",
            "why",
            "src/b.rs",
        ]);
        repo.ok(&["skim", "add", "Cargo.lock", "--reason", "regenerated"]);

        // `--port 0` and read back what the OS gave it. Picking a free port
        // ourselves means binding, releasing, and hoping nobody takes it in
        // between — with these tests running in parallel, that is a race
        // against our own suite.
        let mut args = vec![
            "serve".to_string(),
            "--port".to_string(),
            "0".to_string(),
            "--no-open".to_string(),
        ];
        args.extend(extra.iter().map(|s| s.to_string()));

        let mut child = Command::new(BIN)
            .args(&args)
            .current_dir(repo.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("farol serve should start");

        let port = port_from(child.stdout.take().expect("stdout was piped"));
        let serving = Serving { repo, child, port };
        serving
            .try_get("/api/review")
            .expect("the server should have come up");
        serving
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }

    /// Ask until it answers, or give up.
    fn try_get(&self, path: &str) -> Option<String> {
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            let out = Command::new("curl")
                .args(["-sf", &self.url(path)])
                .output()
                .ok()?;
            if out.status.success() {
                return Some(String::from_utf8_lossy(&out.stdout).into_owned());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        None
    }

    fn get(&self, path: &str) -> String {
        self.try_get(path)
            .unwrap_or_else(|| panic!("GET {path} never answered"))
    }

    fn json(&self, path: &str) -> Value {
        serde_json::from_str(&self.get(path)).expect("the body should be json")
    }

    /// Status code and body, for the requests that are meant to be refused.
    fn probe(&self, method: &str, path: &str, body: Option<&str>) -> (u32, String) {
        let mut args = vec![
            "-s".to_string(),
            "-o".to_string(),
            "/dev/stderr".to_string(),
            "-w".to_string(),
            "%{http_code}".to_string(),
            "-X".to_string(),
            method.to_string(),
        ];
        if let Some(body) = body {
            args.extend([
                "-H".to_string(),
                "content-type: application/json".to_string(),
                "-d".to_string(),
                body.to_string(),
            ]);
        }
        args.push(self.url(path));

        let out = Command::new("curl")
            .args(&args)
            .output()
            .expect("curl runs");
        (
            String::from_utf8_lossy(&out.stdout).trim().parse().unwrap(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }
}

impl Drop for Serving {
    fn drop(&mut self) {
        // `SIGTERM`, not `kill`: the server shuts down cleanly and the process
        // runs its exit handlers. Cutting it down mid-flight would also throw
        // away the coverage it recorded, which is how the drop in these files
        // was first noticed.
        let terminated = Command::new("kill")
            .args(["-TERM", &self.child.id().to_string()])
            .status()
            .is_ok_and(|s| s.success());

        if !terminated {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

/// The port the server actually got, off the line it prints on the way up.
/// Reading it back is the only account that cannot be stale.
fn port_from(stdout: std::process::ChildStdout) -> u16 {
    use std::io::{BufRead, BufReader};

    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        if let Some((_, tail)) = line.rsplit_once(':')
            && let Ok(port) = tail.trim().parse()
        {
            return port;
        }
    }
    panic!("the server never said where it was listening");
}

// ---- what the screen is built from --------------------------------------

#[test]
fn the_review_arrives_in_the_order_the_author_chose() {
    let s = Serving::new();

    let review = s.json("/api/review");

    assert_eq!(review["branch"], "feature/x");
    assert_eq!(review["base"], "main");
    let blocks = review["blocks"].as_array().unwrap();
    assert_eq!(blocks[0]["slug"], "core");
    assert_eq!(blocks[1]["slug"], "wiring");
    assert_eq!(blocks[1]["title"], "The wiring");
}

#[test]
fn every_level_of_prose_the_session_wrote_reaches_the_browser() {
    // Block context, file note and line note are the whole product; if any of
    // them stops at the API the screen is a plain diff viewer.
    let s = Serving::new();

    let review = s.json("/api/review");

    let core = &review["blocks"][0];
    assert_eq!(core["context"], "c");
    let file = &core["files"][0];
    assert_eq!(file["path"], "src/a.rs");
    assert_eq!(file["notes"][0]["text"], "start here");
    assert_eq!(file["lineNotes"][0]["text"], "the actual fix");
    assert_eq!(file["lineNotes"][0]["from"], 10);
    assert_eq!(file["lineNotes"][0]["to"], 12);
}

#[test]
fn a_file_marked_skim_arrives_with_the_reason_it_can_be_skimmed() {
    let s = Serving::new();

    let review = s.json("/api/review");

    let loose = review["looseSkim"].as_array().unwrap();
    assert_eq!(loose[0]["path"], "Cargo.lock");
    assert_eq!(loose[0]["skim"], true);
    assert_eq!(loose[0]["skimReason"], "regenerated");
}

#[test]
fn the_diff_of_a_file_comes_back_with_the_line_numbers_the_notes_anchor_to() {
    let s = Serving::new();

    let diff = s.json("/api/file?path=src/a.rs");

    assert_eq!(diff["path"], "src/a.rs");
    assert_eq!(diff["binary"], false);
    let lines = diff["hunks"][0]["lines"].as_array().unwrap();
    assert!(
        lines.iter().any(|l| l["new_number"] == 10),
        "the line the note points at has to be in the diff"
    );
}

#[test]
fn a_path_outside_the_review_is_refused_rather_than_served() {
    // The pane is driven by the path in the query; without this a crafted
    // request would read any file in the repository.
    let s = Serving::new();

    let (status, body) = s.probe("GET", "/api/file?path=../../etc/passwd", None);

    assert_eq!(status, 400);
    assert!(body.contains("passwd"), "{body}");
}

// ---- what the reviewer leaves behind ------------------------------------

#[test]
fn marking_a_file_read_survives_a_reload() {
    let s = Serving::new();
    assert_eq!(s.json("/api/review")["viewedFiles"], 0);

    let (status, _) = s.probe(
        "POST",
        "/api/viewed",
        Some(r#"{"path":"src/a.rs","viewed":true}"#),
    );
    assert_eq!(status, 204);

    let review = s.json("/api/review");
    assert_eq!(review["viewedFiles"], 1);
    assert_eq!(review["blocks"][0]["files"][0]["viewed"], true);
}

#[test]
fn marking_a_file_read_is_written_to_the_git_dir() {
    // It has to outlive the process, or every restart would start the review
    // over.
    let s = Serving::new();
    s.probe(
        "POST",
        "/api/viewed",
        Some(r#"{"path":"src/a.rs","viewed":true}"#),
    );

    let state = s.repo.path().join(".git/farol/feature-x/state.json");
    let raw = std::fs::read_to_string(&state)
        .unwrap_or_else(|e| panic!("{} should exist: {e}", state.display()));

    assert!(raw.contains("src/a.rs"), "{raw}");
}

#[test]
fn a_tick_can_be_taken_back() {
    let s = Serving::new();
    s.probe(
        "POST",
        "/api/viewed",
        Some(r#"{"path":"src/a.rs","viewed":true}"#),
    );

    s.probe(
        "POST",
        "/api/viewed",
        Some(r#"{"path":"src/a.rs","viewed":false}"#),
    );

    assert_eq!(s.json("/api/review")["viewedFiles"], 0);
}

#[test]
fn marking_a_path_outside_the_review_is_refused() {
    let s = Serving::new();

    let (status, _) = s.probe(
        "POST",
        "/api/viewed",
        Some(r#"{"path":"elsewhere.rs","viewed":true}"#),
    );

    assert_eq!(status, 400);
    assert_eq!(s.json("/api/review")["viewedFiles"], 0);
}

// ---- the page finding out on its own ------------------------------------

#[test]
fn writing_a_map_reaches_the_open_page_without_it_asking() {
    // The reviewer is reading; the skill rewrites the map next door. The page
    // has to find out, or it shows a review that no longer exists.
    let s = Serving::with(&[]);

    let watching = Command::new("curl")
        .args(["-sN", "--max-time", "8", &s.url("/api/watch")])
        .stdout(Stdio::piped())
        .spawn()
        .expect("curl should start");

    std::thread::sleep(Duration::from_millis(500));
    s.repo.ok(&[
        "block",
        "add",
        "later",
        "--title",
        "Later",
        "--context",
        "written while reading",
    ]);

    let out = watching.wait_with_output().expect("curl should finish");
    let body = String::from_utf8_lossy(&out.stdout);

    assert!(body.contains("event: map"), "{body}");
}

// ---- the frontend ---------------------------------------------------------

#[test]
fn the_page_itself_is_served_from_the_binary() {
    let s = Serving::new();

    let out = Command::new("curl")
        .args(["-s", "-D", "-", "-o", "/dev/stderr", &s.url("/")])
        .output()
        .expect("curl runs");
    let headers = String::from_utf8_lossy(&out.stdout);

    if headers.contains("404") {
        // `web/dist` is empty in a fresh checkout; the message has to say so.
        assert!(
            String::from_utf8_lossy(&out.stderr).contains("just build"),
            "an unbuilt frontend has to say what to run"
        );
    } else {
        assert!(headers.contains("text/html"), "{headers}");
    }
}

// ---- refusing to start ----------------------------------------------------

#[test]
fn serve_will_not_start_without_a_map() {
    // Falling back to a plain diff viewer would make farol a worse version of
    // tools that already do that well.
    let repo = Repo::new();
    repo.feature();

    let out = repo.farol(&["serve", "--port", "0", "--no-open", "--no-watch"]);

    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("feature/x"), "{err}");
}

#[test]
fn serve_refuses_a_port_that_is_already_taken() {
    // Two farols on one port is an ordinary mistake; the message has to say
    // which port so the reviewer can pick another.
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.core_block(&["src/a.rs"]);

    let held = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = held.local_addr().unwrap().port();

    let out = repo.farol(&[
        "serve",
        "--port",
        &port.to_string(),
        "--no-open",
        "--no-watch",
    ]);

    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains(&port.to_string()), "{err}");
}
