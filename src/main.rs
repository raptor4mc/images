use axum::{routing::post, Json, Router};
use serde::Deserialize;
use std::env;
use tokio::sync::OnceCell;

static DISCORD_WEBHOOK: OnceCell<String> = OnceCell::const_new();

#[derive(Deserialize)]
struct GitHubEvent {
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    r#ref: Option<String>,
    #[serde(default)]
    repository: Option<Repo>,
    #[serde(default)]
    sender: Option<Sender>,
}

#[derive(Deserialize)]
struct Repo {
    name: String,
}

#[derive(Deserialize)]
struct Sender {
    login: String,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let webhook = env::var("DISCORD_WEBHOOK")
        .expect("Missing DISCORD_WEBHOOK env var");
    DISCORD_WEBHOOK.set(webhook).unwrap();

    let port = env::var("PORT").unwrap_or("8080".into());
    let addr = format!("0.0.0.0:{}", port);

    let app = Router::new().route("/webhook", post(handle_webhook));

    println!("Listening on {}", addr);

    axum::Server::bind(&addr.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handle_webhook(Json(payload): Json<serde_json::Value>) -> &'static str {
    let repo = payload["repository"]["name"].as_str().unwrap_or("unknown");
    let sender = payload["sender"]["login"].as_str().unwrap_or("unknown");
    let event = payload["action"].as_str().unwrap_or("push");

    let msg = format!("📦 **GitHub Event**\nRepo: `{}`\nUser: `{}`\nEvent: `{}`",
        repo, sender, event
    );

    send_to_discord(msg).await;

    "ok"
}

async fn send_to_discord(content: String) {
    let webhook = DISCORD_WEBHOOK.get().unwrap();

    let _ = reqwest::Client::new()
        .post(webhook)
        .json(&serde_json::json!({ "content": content }))
        .send()
        .await;
}
