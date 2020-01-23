use chrono::offset::Utc;
use chrono::serde::ts_seconds;
use chrono::DateTime;
use getopts::{Matches, Options};
use reqwest;
use reqwest::header::HeaderMap;
use reqwest::Client;
use serde;
use serde::Deserialize;
use std::env;
use std::time::Duration;
use tokio::fs::create_dir_all;
use tokio::fs::File;
use tokio::prelude::*;
use tokio::process::Command;
use tokio::time;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.reqopt("t", "", "set app token", "APP_TOKEN");
    opts.optflag("h", "help", "print this help");
    match opts.parse(&args[1..]) {
        Ok(m) if m.opt_present("h") => print_usage(&program, opts),
        // TOOD: print failure reason
        Err(_) => print_usage(&program, opts),
        Ok(m) => start(m).await,
    }
}

async fn start(matches: Matches) {
    if let Some(token) = matches.opt_str("t") {
        let join_handle = tokio::spawn(async move {
            println!("starting schedules runner with token: {:?}", token);
            let mut interval = time::interval(Duration::from_secs(10));

            let mut headers = HeaderMap::new();
            headers.insert("token", token.parse().unwrap());
            let client = Client::builder().default_headers(headers).build().unwrap();
            loop {
                interval.tick().await;
                let execs = query_execs(&client).await;
                for e in execs {
                    execute(e);
                }
            }
        });
        join_handle.await.unwrap();
    }
}

#[derive(Debug, Deserialize)]
struct Task {
    id: String,
    name: String,
    script: String,
}

#[derive(Debug, Deserialize)]
struct Execution {
    id: String,
    task: Task,
    #[serde(with = "ts_seconds")]
    start_time: DateTime<Utc>,
}

async fn query_execs(client: &Client) -> Vec<Execution> {
    let r = client
        .get("http://localhost:8080/api/v1/execs")
        .send()
        .await;
    match r {
        Ok(resp) if resp.status().is_success() => resp.json::<Vec<Execution>>().await.unwrap(),
        Ok(resp) => {
            let status = resp.status();
            let content = resp.text().await;
            panic!("invalid response: {}, {:?}", status, content);
        }
        Err(e) => panic!("query executions failed: {}", e),
    }
}

fn execute(exec: Execution) {
    tokio::spawn(async move {
        println!("running execution {:?}", exec);
        let dir = format!("/tmp/tasks/{}/execs/{}", exec.task.id, exec.id);
        println!("create directory for this execution: {}", dir);
        create_dir_all(dir).await.unwrap();

        let task_file = &format!("/tmp/tasks/{}/execs/{}/task.sh", exec.task.id, exec.id);
        let mut file = File::create(task_file.as_str()).await.unwrap();
        file.write_all(exec.task.script.as_bytes()).await.unwrap();

        let child = Command::new("bash").arg(task_file.as_str()).spawn();
        child.expect("failed to spawn").await.unwrap();
    });
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] -t APP_TOKEN", program);
    print!("{}", opts.usage(&brief));
}
