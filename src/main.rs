use serde::Deserialize;
use async_std::task;
use std::env;
use std::time::{Duration, SystemTime};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = sentry::init(("https://0fe0d16e158146279a751bbf675f2610@o117736.ingest.sentry.io/5536978", sentry::ClientOptions {
        debug: true,
        attach_stacktrace: true,
        ..Default::default()
    }));

    // TODO: Configurable
    let interval: Duration = Duration::from_secs(15);
    let tag = "sentry";
    let max_items = 3;
    let key: Option<String> = match env::var("OXIDEOVERFLOW_STACKOVERFLOW_KEY") {
        Ok(v) => Some(v),
        Err(_) => None
    };
    let discord_url = env::var("OXIDEOVERFLOW_DISCORD_URL").unwrap();

    sentry::configure_scope(|scope| {
        scope.set_tag("stackoverflow.tag", tag);
        scope.set_tag("stackoverflow.max_items", max_items);
        scope.set_tag("stackoverflow.has_key", !key.is_none());
        scope.set_tag("interval", interval.as_secs());
        scope.set_tag("discord.url", discord_url.as_str());
    });

    let mut offset: Option<Duration> = None;
    let mut iterations = 0;
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

        iterations = iterations + 1;
        
        sentry::configure_scope(|scope| {
            scope.set_extra("stackoverflow.from", from.as_secs().into());
            scope.set_extra("stackoverflow.to", to.as_secs().into());
            scope.set_extra("iterations", iterations.into());
        });
        println!("Waiting for {} seconds before polling.", interval.as_secs());

        task::sleep(interval).await;

        println!("Fetching from {}", stackoverflow_url);

        match reqwest::get(&stackoverflow_url).await {
            Ok(response) => {
                let status = response.status();
                println!("Status: {}", status);
                if status == 200 {
                    match response.json().await {
                        Ok(r) => {
                            let response: Response = r;
                            println!("Response: {:#?}", response);
                            handle_response(response, discord_url.as_str());
                        },
                        Err(e) => {
                            println!("Response error {}", e);
                            sentry::capture_error(&e);
                        }
                    };
                } else {
                    if let Ok(e) = response.text().await {
                        sentry::with_scope(|scope| {
                            scope.set_tag("http.status", status);
                        }, || {
                            sentry::capture_message(&e, sentry::Level::Error);
                        });
                        println!("Payload: {:#?}", e);
                    } else {
                        sentry::capture_message(
                            format!("Call failed with status {} and no body.", status).as_str(), 
                            sentry::Level::Error);
                    }
                }
            }
            Err(e) => println!("Failed with error: {}, on stackoverflow_url: {}", e, stackoverflow_url),
        }
    }
}

fn handle_response(response: Response, discord_url: &str) {
    for item in response.items.iter() {
        println!("item: {:#?}", item);

        sentry::add_breadcrumb(sentry::Breadcrumb {
            category: Some("stackoverflow".into()),
            message: Some(format!("Processing question {}", item.title)),
            ..Default::default()
        });

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
