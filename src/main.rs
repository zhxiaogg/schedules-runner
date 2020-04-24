use chrono::offset::Utc;
use chrono::serde::ts_seconds;
use chrono::DateTime;
use getopts::{Matches, Options};
use reqwest;
use reqwest::header::HeaderMap;
use reqwest::Client;
use reqwest::Response;
use serde;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;
use tokio::fs::create_dir_all;
use tokio::fs::File;
use tokio::prelude::*;
use tokio::process::Command;
use tokio::time;
mod settings;
use settings::Settings;
use std::sync::Arc;

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
        headers.insert("runner", "Client".parse().unwrap());
        let client = Arc::new(Client::builder().default_headers(headers).build().unwrap());
        loop {
            interval.tick().await;
            let execs = query_execs(&settings, client.clone()).await;
            for e in execs {
                execute(&settings, e, client.clone());
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
    payload: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Execution {
    id: String,
    task: Task,
    #[serde(with = "ts_seconds")]
    start_time: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct UpdateResult {
    result: bool,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Serialize)]
struct ExecutionStatus {
    value: String,
    idempotent_key: String,
}

#[derive(Debug, Serialize)]
struct ExecutionUpdateView {
    status: ExecutionStatus,
}

async fn query_execs(settings: &Settings, client: Arc<Client>) -> Vec<Execution> {
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

fn execute(settings: &Settings, exec: Execution, client: Arc<Client>) {
    let url = format!("{}/api/v1/execs/{}", settings.server, exec.id);
    let exec_dir = format!("{}tasks/{}/execs/{}", settings.logs, exec.task.id, exec.id);
    tokio::spawn(async move {
        let update = ExecutionUpdateView {
            status: ExecutionStatus {
                value: "Started".to_owned(),
                idempotent_key: "test".to_owned(),
            },
        };
        let response = client.post(url.as_str()).json(&update).send().await;
        if update_success(response).await {
            println!("running execution {:?}", exec);
            println!("create directory for this execution: {}", exec_dir);
            create_dir_all(exec_dir.as_str()).await.unwrap();

            let task_file_path = &format!("{}/task.sh", exec_dir);
            let mut task_file = File::create(task_file_path.as_str()).await.unwrap();
            task_file
                .write_all(exec.task.payload.as_bytes())
                .await
                .unwrap();

            let child = Command::new("bash").arg(task_file_path.as_str()).spawn();
            child.expect("failed to spawn").await.unwrap();
        } else {
            println!("cannot mark exec as started, will giveup execution.");
        }
    });
}

async fn update_success(response: reqwest::Result<Response>) -> bool {
    match response {
        Ok(resp) if resp.status().is_success() => {
            let r = resp.json::<UpdateResult>().await.unwrap();
            r.result
        }
        _ => false,
    }
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] -t APP_TOKEN", program);
    print!("{}", opts.usage(&brief));
}
