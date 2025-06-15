use tokio::io;
use tokio::task;
use axum::{routing::{get,post}, Router,extract::Request,http::StatusCode,body};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::process::Command;
use reqwest;
mod task_manager;

fn print_usage() {
    println!("使い方:");
    println!("  multi-task-pwsh run <command>  - コマンドをバックグラウンドで実行");
    println!("  multi-task-pwsh list           - 実行中および完了したタスクを一覧表示");
    println!("  multi-task-pwsh status <id>    - 特定のタスクのステータスを表示");
    println!("  multi-task-pwsh log <id>       - タスクのログを表示");
}

async fn run_task(request: Request) -> (StatusCode, &'static str) {
    let body = body::to_bytes(request.into_body(), usize::MAX).await;
    let command = match body {
        Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
        Err(_) => return (StatusCode::BAD_REQUEST, "Failed to read request body"),
    };
    if command.is_empty() {
        return (StatusCode::BAD_REQUEST, "Command cannot be empty");
    }
    println!("Running command: {}", command);
    // コマンドをバックグラウンドで実行
    let mut child = Command::new("pwsh")
        .arg("-Command")
        .arg(command)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn().unwrap();
    // プロセスの完了を待機しない
    let _ = child.wait();
    // 成功レスポンスを返す
    
    (StatusCode::OK, "Task started")
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    let subcommand = if args.len() < 2 { "_" } else { &args[1] };
    
    match subcommand {
        "stop" => {}
        "run" => {
            if args.len() < 3 {
                println!("コマンドを指定してください");
                print_usage();
                return;
            }
            let client = reqwest::Client::new();
            let command = args[2..].join(" ");
            let response = client.post("http://localhost:51890/run")
                .body(command)
                .send().await;
            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        println!("コマンドが正常に実行されました");
                    } else {
                        println!("コマンドの実行に失敗しました: {}", resp.status());
                    }
                },
                Err(e) => {
                    println!("リクエストの送信に失敗しました: {}", e);
                }
            }
        }
        _ => {
            // WebサーバーとPowerShellを並行して起動
            let web_task = task::spawn(async {
                let app = Router::new()
                    .route("/", get(|| async { "Multi-task PowerShell Web Interface" }))
                    .route("/status", get(|| async { "Server is running" }))
                    .route("/run", post(run_task));

                let addr = SocketAddr::from(([127, 0, 0, 1], 51890));
                let listener = TcpListener::bind(addr).await.unwrap();
                println!("Webサーバーが http://localhost:51890 で起動しました");
                
                axum::serve(listener, app).await.unwrap();
            });

            let pwsh_task = task::spawn(async move {
                let mut child = Command::new("pwsh")
                    .arg("-Interactive")
                    .stdin(std::process::Stdio::inherit())
                    .stdout(std::process::Stdio::inherit())
                    .stderr(std::process::Stdio::inherit())
                    .spawn().unwrap();

                // プロセスの完了を待機
                let status = child.wait().await.unwrap();
                println!("プロセス終了: {:?}", status);
            });

            // 両方のタスクを並行実行
            tokio::select! {
                _ = web_task => {
                    println!("Webサーバーが終了しました");
                    std::process::exit(0);
                },
                _ = pwsh_task => {
                    println!("PowerShellが終了しました");
                    std::process::exit(0);
                },
            }
        }
    };
    return
}