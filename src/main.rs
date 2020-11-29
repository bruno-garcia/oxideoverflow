use serde::Deserialize;
use async_std::task;
use std::env;
use std::time::{Duration, SystemTime};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = sentry::init("https://0fe0d16e158146279a751bbf675f2610@o117736.ingest.sentry.io/5536978");

    let interval: Duration = Duration::from_secs(15);
    let now = SystemTime::now();

    let query = "[sentry]";
    let key = env::var("OXIDEOVERFLOW_STACKOVERFLOW_KEY").unwrap();

    loop {
        let from = now.duration_since(SystemTime::UNIX_EPOCH)?;
        let to = now.checked_add(interval).unwrap().duration_since(SystemTime::UNIX_EPOCH)?;
        let stackoverflow_url = get_url(from, to, query, key.as_str());
        task::sleep(interval).await;

        println!("Fetching from {}", stackoverflow_url);

        match reqwest::get(&stackoverflow_url).await {
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
            Err(e) => println!("Failed with error: {}, on stackoverflow_url: {}", e, stackoverflow_url),
        }
    }
}

fn get_url(from: Duration, to: Duration, query: &str, key: &str) -> String {
    format!("https://api.stackexchange.com/2.2/questions?\
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
        from.as_secs(),
        to.as_secs(),
        query,
        key)
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
