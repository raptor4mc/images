use axum::{http::HeaderMap, routing::post, Json, Router};
use std::env;
use tokio::sync::OnceCell;

static DISCORD_WEBHOOK: OnceCell<String> = OnceCell::const_new();

const RAP_THUMBNAIL_URL: &str =
    "https://upload.wikimedia.org/wikipedia/commons/thumb/2/26/Microphone.svg/240px-Microphone.svg.png";

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let webhook = env::var("DISCORD_WEBHOOK").expect("Missing DISCORD_WEBHOOK env var");
    DISCORD_WEBHOOK.set(webhook).unwrap();

    let port = env::var("PORT").unwrap_or("8080".into());
    let addr = format!("0.0.0.0:{}", port);

    let app = Router::new().route("/webhook", post(handle_webhook));

    println!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn handle_webhook(
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> &'static str {
    let github_event = headers
        .get("x-github-event")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown");

    let embed = build_embed(github_event, &payload);
    send_to_discord(embed).await;

    "ok"
}

fn build_embed(event_type: &str, payload: &serde_json::Value) -> serde_json::Value {
    let repo = payload["repository"]["name"].as_str().unwrap_or("unknown");
    let sender = payload["sender"]["login"].as_str().unwrap_or("unknown");
    let action = payload["action"].as_str().unwrap_or("-");

    let mut title = String::from("Unknown GitHub Event");
    let mut description = format!("`{}` triggered an unsupported event in `{}`.", sender, repo);
    let mut url = payload["repository"]["html_url"]
        .as_str()
        .unwrap_or("https://github.com")
        .to_string();
    let mut color = 0x6B7280;
    let mut fields = vec![
        serde_json::json!({ "name": "Repository", "value": repo, "inline": true }),
        serde_json::json!({ "name": "Sender", "value": sender, "inline": true }),
        serde_json::json!({ "name": "Event", "value": event_type, "inline": true }),
    ];

    match event_type {
        "push" => {
            let branch = payload["ref"]
                .as_str()
                .unwrap_or("unknown")
                .trim_start_matches("refs/heads/");
            let commit_count = payload["commits"]
                .as_array()
                .map_or(0, |commits| commits.len());

            title = "🚀 Push Event".to_string();
            description = format!(
                "`{}` pushed **{}** commit(s) to `{}`.",
                sender, commit_count, branch
            );
            url = payload["compare"].as_str().unwrap_or(&url).to_string();
            color = 0x2ECC71;
            fields.push(serde_json::json!({ "name": "Branch", "value": branch, "inline": true }));
            fields.push(serde_json::json!({ "name": "Commits", "value": commit_count.to_string(), "inline": true }));
        }
        "pull_request" => {
            let number = payload["number"].as_i64().unwrap_or_default();
            let pr_title = payload["pull_request"]["title"]
                .as_str()
                .unwrap_or("Untitled pull request");

            title = "🔀 Pull Request Event".to_string();
            description = format!(
                "PR #{} `{}` was **{}** by `{}`.",
                number, pr_title, action, sender
            );
            url = payload["pull_request"]["html_url"]
                .as_str()
                .unwrap_or(&url)
                .to_string();
            color = 0x3498DB;
            fields.push(serde_json::json!({ "name": "Action", "value": action, "inline": true }));
        }
        "issues" => {
            let number = payload["issue"]["number"].as_i64().unwrap_or_default();
            let issue_title = payload["issue"]["title"]
                .as_str()
                .unwrap_or("Untitled issue");

            title = "🐛 Issue Event".to_string();
            description = format!(
                "Issue #{} `{}` was **{}** by `{}`.",
                number, issue_title, action, sender
            );
            url = payload["issue"]["html_url"]
                .as_str()
                .unwrap_or(&url)
                .to_string();
            color = 0xE67E22;
            fields.push(serde_json::json!({ "name": "Action", "value": action, "inline": true }));
        }
        "release" => {
            let tag = payload["release"]["tag_name"].as_str().unwrap_or("unknown");
            let name = payload["release"]["name"]
                .as_str()
                .unwrap_or("Unnamed release");

            title = "📦 Release Event".to_string();
            description = format!("Release **{}** (`{}`) was **{}**.", name, tag, action);
            url = payload["release"]["html_url"]
                .as_str()
                .unwrap_or(&url)
                .to_string();
            color = 0x9B59B6;
            fields.push(serde_json::json!({ "name": "Tag", "value": tag, "inline": true }));
        }
        "status" => {
            let state = payload["state"].as_str().unwrap_or("unknown");
            let sha = payload["sha"].as_str().unwrap_or("-");
            let short_sha: String = sha.chars().take(7).collect();

            title = "✅ Status Event".to_string();
            description = format!("Commit `{}` status changed to **{}**.", short_sha, state);
            url = payload["target_url"].as_str().unwrap_or(&url).to_string();
            color = 0x1ABC9C;
            fields.push(serde_json::json!({ "name": "State", "value": state, "inline": true }));
        }
        _ => {}
    }

    serde_json::json!({
        "embeds": [
            {
                "title": title,
                "description": description,
                "url": url,
                "color": color,
                "thumbnail": { "url": RAP_THUMBNAIL_URL },
                "fields": fields
            }
        ]
    })
}

async fn send_to_discord(payload: serde_json::Value) {
    let webhook = DISCORD_WEBHOOK.get().unwrap();

    let _ = reqwest::Client::new()
        .post(webhook)
        .json(&payload)
        .send()
        .await;
}
