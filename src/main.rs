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
mod settings;
use settings::Settings;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.reqopt("t", "", "set app token", "APP_TOKEN");
    opts.optflag("h", "help", "print this help");
    match opts.parse(&args[1..]) {
        Err(_) => print_usage(&program, opts),
        Ok(m) if m.opt_present("h") => print_usage(&program, opts),
        Ok(m) => start(m).await,
    }
}

async fn start(matches: Matches) {
    let token = matches.opt_str("t");
    let settings = Settings::new(token).unwrap();
    tokio::spawn(async move {
        let token = settings.token.as_str();
        println!("starting schedules runner with token: {:?}", token);
        let mut interval = time::interval(Duration::from_secs(10));

        let mut headers = HeaderMap::new();
        headers.insert("token", token.parse().unwrap());
        let client = Client::builder().default_headers(headers).build().unwrap();
        loop {
            interval.tick().await;
            let execs = query_execs(&settings, &client).await;
            for e in execs {
                execute(&settings, e);
            }
        }
    })
    .await
    .unwrap();
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

async fn query_execs(settings: &Settings, client: &Client) -> Vec<Execution> {
    let url = format!("{}/api/v1/execs", settings.server);
    let r = client.get(url.as_str()).send().await;
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

fn execute(settings: &Settings, exec: Execution) {
  let exec_dir = format!("{}tasks/{}/execs/{}", settings.logs, exec.task.id, exec.id);
    tokio::spawn(async move {
        println!("running execution {:?}", exec);
        println!("create directory for this execution: {}", exec_dir);
        create_dir_all(exec_dir.as_str()).await.unwrap();

        let task_file_path = &format!("{}/task.sh", exec_dir);
        let mut task_file = File::create(task_file_path.as_str()).await.unwrap();
        task_file.write_all(exec.task.script.as_bytes()).await.unwrap();

        let child = Command::new("bash").arg(task_file_path.as_str()).spawn();
        child.expect("failed to spawn").await.unwrap();
    });
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] -t APP_TOKEN", program);
    print!("{}", opts.usage(&brief));
}
