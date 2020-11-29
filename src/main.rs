use serde::Deserialize;
use async_std::task;
use std::env;
use std::time::{Duration, SystemTime};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let now = SystemTime::now();
    let from = now.duration_since(SystemTime::UNIX_EPOCH)?.as_secs();
    let to = now.checked_add(Duration::from_secs(15)).unwrap().duration_since(SystemTime::UNIX_EPOCH)?.as_secs();

    let query = "[sentry]";
    let key = env::var("OXIDEOVERFLOW_STACKOVERFLOW_KEY").unwrap();

    let path = format!("https://api.stackexchange.com/2.2/questions?\
        page=1&\
        pagesize=2&\
        order=asc&\
        sort=creation&\
        tagged=sentry&\
        site=stackoverflow&\
        fromdate={}&\
        todate={}&\
        q={}&\
        key={}",
        from,
        to,
        query,
        key);

    println!("{}", path);
    match reqwest::get(&path).await {
        Ok(response) => {
            println!("Status: {}", response.status());
            if response.status() == 200 {
                match response.json().await {
                    Ok(r) => {
                        let json: Response = r;
                        println!("json: {:#?}", json);
                    },
                    Err(e) => println!("err {}", e),
                };
            } else {
                println!("Payload: {:#?}", response.text().await?);
            }
        }
        Err(e) => println!("Failed with error: {}, on url: {}", e, path),
    }
    // To make the worker
    task::sleep(Duration::from_secs(10)).await;
    Ok(())
}

#[derive(Deserialize, Debug)]
struct Response {
    has_more: bool,
    quota_max: u32,
    quota_remaining: u32,
    items: Vec<Question>,
}

#[derive(Deserialize, Debug)]
struct Question {
    title: String,
    owner: Owner,
    tags: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct Owner {
    reputation: u64,
    user_id: u64,
    user_type: String,
    accept_rate: Option<u32>,
    profile_image: String,
    display_name: String,
    link: String,
}
