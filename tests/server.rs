//! The HTTP surface, against a real server.
//!
//! Driven over the wire rather than by handing the router a request: the
//! browser talks to a process, and everything between the socket and the map on
//! disk — the composition root, the listener, git, the store — is part of what
//! can break. Injecting fakes underneath would test the handlers and vouch for
//! none of that.

mod common;

use common::{Repo, numbered};
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
        let repo = mapped();
        // No `--port` at all: this is how a reviewer starts one, and it
        // exercises the search. The port comes back off the line the server
        // prints, which is the only account that cannot be stale.
        //
        // `--foreground`, so this test owns the process it is talking to: the
        // detached start is a different thing and is tested as one, further
        // down.
        let mut args = vec![
            "serve".to_string(),
            "--no-open".to_string(),
            "--foreground".to_string(),
        ];
        args.extend(extra.iter().map(|s| s.to_string()));

        let mut child = repo
            .command(&args.iter().map(String::as_str).collect::<Vec<_>>())
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

/// A branch with a map worth serving: two blocks, all three levels of prose,
/// and something marked skim.
fn mapped() -> Repo {
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
    repo
}

/// The port the server actually got, off the line it prints on the way up.
/// Reading it back is the only account that cannot be stale.
fn port_from(stdout: std::process::ChildStdout) -> u16 {
    use std::io::{BufRead, BufReader};

    BufReader::new(stdout)
        .lines()
        .map_while(Result::ok)
        .find_map(|line| port_in(&line))
        .expect("the server never said where it was listening")
}

fn port_in(line: &str) -> Option<u16> {
    line.rsplit_once(':')
        .and_then(|(_, tail)| tail.trim().parse().ok())
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

// ---- what the reviewer writes back --------------------------------------

#[test]
fn a_comment_written_from_the_page_can_be_read_closed_and_dropped() {
    // The whole life of a comment over the wire, in one go: each step is only
    // worth anything if the one before it stuck.
    let s = Serving::new();
    assert_eq!(s.json("/api/comments").as_array().unwrap().len(), 0);

    let (status, body) = s.probe(
        "POST",
        "/api/comments",
        Some(r#"{"path":"src/a.rs","from":10,"to":12,"body":"Why this order?"}"#),
    );
    assert_eq!(status, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .expect("the new comment comes back with the id to address it by")
        .to_string();

    let all = s.json("/api/comments");
    assert_eq!(all[0]["path"], "src/a.rs");
    assert_eq!(all[0]["from"], 10);
    assert_eq!(all[0]["to"], 12);
    assert_eq!(all[0]["body"], "Why this order?");
    assert_eq!(all[0]["resolved"], false);

    let (status, _) = s.probe(
        "POST",
        &format!("/api/comments/{id}/resolve"),
        Some(r#"{"resolved":true}"#),
    );
    assert_eq!(status, 200);
    let all = s.json("/api/comments");
    assert_eq!(all.as_array().unwrap().len(), 1, "closed is not deleted");
    assert_eq!(all[0]["resolved"], true);

    let (status, _) = s.probe("DELETE", &format!("/api/comments/{id}"), None);
    assert_eq!(status, 204);
    assert_eq!(s.json("/api/comments").as_array().unwrap().len(), 0);
}

#[test]
fn a_comment_is_written_to_the_git_dir_as_markdown() {
    // The file is the point: it outlives the server, and it is meant to be
    // opened in an editor.
    let s = Serving::new();
    s.probe(
        "POST",
        "/api/comments",
        Some(r#"{"path":"src/a.rs","from":10,"to":12,"body":"Why this order?"}"#),
    );

    let dir = s.repo.path().join(".git/farol/feature-x/comments");
    let written = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{} should exist: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .collect::<Vec<_>>();

    assert_eq!(written.len(), 1);
    assert_eq!(written[0].extension().unwrap(), "md");
    let raw = std::fs::read_to_string(&written[0]).unwrap();
    assert!(raw.contains("path: src/a.rs"), "{raw}");
    assert!(raw.contains("lines: 10-12"), "{raw}");
    assert!(raw.contains("Why this order?"), "{raw}");
}

#[test]
fn a_comment_the_review_cannot_hold_is_refused() {
    // Both ways of asking for a line that is not there: a file outside the
    // window, and a span past the end of one inside it. Either would render
    // nowhere.
    let s = Serving::new();

    let (outside, _) = s.probe(
        "POST",
        "/api/comments",
        Some(r#"{"path":"elsewhere.rs","from":1,"to":1,"body":"Why?"}"#),
    );
    let (past_end, _) = s.probe(
        "POST",
        "/api/comments",
        Some(r#"{"path":"src/a.rs","from":9000,"to":9001,"body":"Why?"}"#),
    );

    assert_eq!(outside, 400);
    assert_eq!(past_end, 400);
    assert_eq!(s.json("/api/comments").as_array().unwrap().len(), 0);
}

#[test]
fn addressing_a_comment_that_is_not_there_is_refused_rather_than_ignored() {
    let s = Serving::new();

    let (resolve, _) = s.probe(
        "POST",
        "/api/comments/nope/resolve",
        Some(r#"{"resolved":true}"#),
    );
    let (remove, _) = s.probe("DELETE", "/api/comments/nope", None);

    assert_eq!(resolve, 400);
    assert_eq!(remove, 400);
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
    // The data line is what makes it an event at all. A message with an empty
    // data buffer is dropped by the browser instead of dispatched, so without
    // this the nudge arrives on the wire, satisfies curl, and never reaches the
    // page — which is exactly how this went unnoticed.
    assert!(body.contains("data: map"), "{body}");
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
fn two_reviews_can_be_open_at_once() {
    // One per worktree is the ordinary case, so the second must find its own
    // port rather than colliding with the first.
    let first = Serving::new();
    let second = Serving::new();

    assert_ne!(first.port, second.port);
    assert!(first.get("/api/review").contains("feature/x"));
    assert!(second.get("/api/review").contains("feature/x"));
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

// ---- servers, and the terminal you get back -----------------------------

/// A review started the way a reviewer starts one: the command returns, and the
/// server stays.
struct Detached {
    repo: Repo,
    port: u16,
}

impl Detached {
    fn new() -> Self {
        let repo = mapped();
        let port = Self::start(&repo);
        Detached { repo, port }
    }

    /// Start one and read back the port it announced.
    fn start(repo: &Repo) -> u16 {
        let out = repo.farol(&["serve", "--no-open", "--no-watch"]);
        assert!(
            out.status.success(),
            "serve should have returned:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let said = String::from_utf8_lossy(&out.stdout).into_owned();
        said.lines()
            .find_map(port_in)
            .unwrap_or_else(|| panic!("serve should say where it landed:\n{said}"))
    }

    fn answering(&self, port: u16) -> bool {
        Command::new("curl")
            .args(["-sf", &format!("http://127.0.0.1:{port}/api/review")])
            .output()
            .is_ok_and(|o| o.status.success())
    }

    /// The process behind a port, read out of the registry.
    ///
    /// Whether a *port* is free proves nothing here: these tests run alongside
    /// each other and the next server to start takes whatever was let go. The
    /// claim worth making is about the process that was asked to stop.
    fn pid_on(&self, port: u16) -> u32 {
        let file = self.repo.state_dir().join(format!("servers/{port}.json"));
        let raw = std::fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("{} should be registered: {e}", file.display()));
        serde_json::from_str::<Value>(&raw).unwrap()["pid"]
            .as_u64()
            .expect("an entry carries the pid") as u32
    }
}

/// Whether a process is still there, asked the way `stop` asks.
fn running(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output()
        .is_ok_and(|o| o.status.success())
}

impl Drop for Detached {
    fn drop(&mut self) {
        // Nothing else will: the process is not this test's child any more,
        // which is the whole point of it.
        let _ = self.repo.farol(&["servers", "stop", "--all"]);
    }
}

#[test]
fn serve_hands_the_terminal_back_and_leaves_the_review_open() {
    let d = Detached::new();

    assert!(
        d.answering(d.port),
        "the server should still be serving after the command that started it returned"
    );
}

#[test]
fn a_server_that_is_running_can_be_found_from_anywhere() {
    // The point of the registry: you are in another directory, or another
    // terminal, and you want to know what is open.
    let d = Detached::new();

    let list = d.repo.ok(&["servers"]);

    assert!(list.contains(&d.port.to_string()), "{list}");
    assert!(list.contains("feature/x → main"), "{list}");
    assert!(
        list.contains(&d.repo.path().to_string_lossy().to_string()),
        "{list}"
    );
}

#[test]
fn asking_for_the_same_review_twice_hands_back_the_one_already_open() {
    // Otherwise every `farol serve` leaves another server behind, and the
    // reviewer ends up with a row of tabs showing the same thing.
    let d = Detached::new();

    let again = Detached::start(&d.repo);

    assert_eq!(again, d.port);
}

#[test]
fn stopping_a_server_ends_it_and_takes_it_off_the_list() {
    let d = Detached::new();
    let pid = d.pid_on(d.port);

    d.repo.ok(&["servers", "stop", &d.port.to_string()]);

    assert!(!running(pid), "the server should have been stopped");
    assert!(d.repo.ok(&["servers"]).contains("No farol server"));
}

#[test]
fn stopping_them_all_reaches_reviews_from_other_repositories() {
    // The registry belongs to the machine, not to a repository: `--all` from
    // one checkout has to stop the review open in another.
    let first = Detached::new();
    let registry = first.repo.state_dir();
    let second = mapped();
    let out = second.farol_sharing(&registry, &["serve", "--no-open", "--no-watch"]);
    assert!(out.status.success());
    let second_port = String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(port_in)
        .expect("the second server should say where it landed");

    let stopped = first.repo.ok(&["servers", "stop", "--all"]);

    assert!(stopped.contains(&first.port.to_string()), "{stopped}");
    assert!(stopped.contains(&second_port.to_string()), "{stopped}");
    assert!(first.repo.ok(&["servers"]).contains("No farol server"));
}

#[test]
fn stopping_a_port_with_nothing_on_it_says_so_instead_of_pretending() {
    let repo = mapped();

    let err = repo.fails(&["servers", "stop", "65000"]);

    assert!(err.contains("65000"), "{err}");
}

#[test]
fn stopping_without_saying_what_refuses_rather_than_guessing() {
    // `--all` has to be typed. Stopping every review on the machine is not
    // something to infer from a bare command.
    let repo = mapped();

    let err = repo.fails(&["servers", "stop"]);

    assert!(err.contains("--all"), "{err}");
}

#[test]
fn nothing_running_is_an_answer_rather_than_silence() {
    let repo = mapped();

    assert!(
        repo.ok(&["servers"])
            .contains("No farol server is running.")
    );
}

#[test]
fn a_server_stopped_from_outside_takes_itself_off_the_list() {
    // Not through `farol servers stop`: a Ctrl-C, or a kill from a script.
    // Going out on its own is what graceful shutdown buys, and the difference
    // shows here — the file is gone the moment the process is, rather than
    // waiting for the next reader to sweep it up.
    let d = Detached::new();
    let pid = d.pid_on(d.port);
    let entry = d.repo.state_dir().join(format!("servers/{}.json", d.port));

    Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status()
        .expect("kill runs");

    let deadline = Instant::now() + Duration::from_secs(5);
    while running(pid) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }

    assert!(!running(pid), "it should have stopped");
    assert!(
        !entry.exists(),
        "it should have taken its own entry out on the way"
    );
}

#[test]
fn a_server_that_stopped_answering_stops_being_listed() {
    // Alive is not the same as serving. A server can stop listening without
    // ending — freezing it is the cheapest way to arrange that — and until the
    // list asks the port, it goes on reporting a review nobody can open.
    let d = Detached::new();
    let pid = d.pid_on(d.port);
    assert!(d.repo.ok(&["servers"]).contains(&d.port.to_string()));

    Command::new("kill")
        .args(["-STOP", &pid.to_string()])
        .status()
        .expect("kill runs");

    let list = d.repo.ok(&["servers"]);

    assert!(
        running(pid),
        "it is still a process, it just does not answer"
    );
    assert!(!list.contains(&d.port.to_string()), "{list}");

    // And it is still reachable, because the entry was left where `stop` looks:
    // a list that hides a server must not also strand it.
    Command::new("kill")
        .args(["-CONT", &pid.to_string()])
        .status()
        .expect("kill runs");
    d.repo.ok(&["servers", "stop", &d.port.to_string()]);
    assert!(!running(pid));
}

#[test]
fn the_health_route_says_which_server_is_answering() {
    // What the list compares against. Without the process id, a port taken by
    // somebody else's review would pass for yours.
    let d = Detached::new();

    let answered = Command::new("curl")
        .args(["-sf", &format!("http://127.0.0.1:{}/health", d.port)])
        .output()
        .expect("curl runs");
    let health: Value =
        serde_json::from_slice(&answered.stdout).expect("health should answer json");

    assert_eq!(health["port"], d.port);
    assert_eq!(health["pid"], d.pid_on(d.port));
    assert_eq!(health["branch"], "feature/x");
}
