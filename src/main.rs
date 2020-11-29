use serde::Deserialize;
use async_std::task;
use std::env;
use std::time::{Duration, SystemTime};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = sentry::init("https://0fe0d16e158146279a751bbf675f2610@o117736.ingest.sentry.io/5536978");

    // TODO: Configurable
    let interval: Duration = Duration::from_secs(15);
    let tag = "sentry";
    let max_items = 3;
    let key: Option<String> = match env::var("OXIDEOVERFLOW_STACKOVERFLOW_KEY") {
        Ok(v) => Some(v),
        Err(_) => None
    };

    sentry::configure_scope(|scope| {
        scope.set_tag("stackoverflow.tag", tag);
        scope.set_tag("stackoverflow.max_items", max_items);
        scope.set_tag("stackoverflow.has_key", !key.is_none());
        scope.set_tag("interval", interval.as_secs());
    });

    let mut offset: Option<Duration> = None;
    loop {
        let now = SystemTime::now();
        let from = match offset {
            // From the last run's 'end' timestamp
            Some(b) => b,
            // From now
            None => now.duration_since(SystemTime::UNIX_EPOCH)?
        };
        let to = now.checked_add(interval).unwrap().duration_since(SystemTime::UNIX_EPOCH)?;
        offset = Some(to);
        let stackoverflow_url = get_url(&from, &to, tag, max_items, &key);

        let task = task::sleep(interval);
        
        sentry::configure_scope(|scope| {
            scope.set_extra("stackoverflow.from", from.as_secs().into());
            scope.set_extra("stackoverflow.to", to.as_secs().into());
        });
        println!("Waiting for {} seconds before polling.", interval.as_secs());

        task.await;

        println!("Fetching from {}", stackoverflow_url);

        match reqwest::get(&stackoverflow_url).await {
            Ok(response) => {
                println!("Status: {}", response.status());
                if response.status() == 200 {
                    match response.json().await {
                        Ok(r) => {
                            let response: Response = r;
                            println!("Response: {:#?}", response);
                            handle_response(response);
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

fn handle_response(response: Response) {
    // https://discord.com/api/webhooks/782664118128869406/ic-s9pKgnQlWWesoQnwBQ9cgpum7EPH_Z64W3sEUJVUZ7WoF1zvX353tLKC123s-Ss3s
    for item in response.items.iter() {
        println!("item: {:#?}", item);
    }
    println!("Done processing response.");
}

fn get_url(from: &Duration, to: &Duration, tag: &str, max_items: u8, key: &Option<String>) -> String {
    let url = format!("https://api.stackexchange.com/2.2/questions?\
        page=1&\
        order=asc&\
        sort=creation&\
        site=stackoverflow&\
        pagesize={}&\
        fromdate={}&\
        todate={}&\
        tagged={}",
        max_items,
        from.as_secs(),
        to.as_secs(),
        tag);
    if let Some(k) = key {
        format!("{}&key={}", url, k)
    } else {
        url
    }
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
