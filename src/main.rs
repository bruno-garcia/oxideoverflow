use serde::Deserialize;
use async_std::task;
use std::env;
use std::time::{Duration, SystemTime};
use webhook::Webhook;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = sentry::init(("https://0fe0d16e158146279a751bbf675f2610@o117736.ingest.sentry.io/5536978", sentry::ClientOptions {
        debug: true,
        attach_stacktrace: true,
        in_app_include: vec!["oxideoverflow"],
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
    let webhook = Webhook::from_url(discord_url.as_str());

    sentry::configure_scope(|scope| {
        scope.set_tag("stackoverflow.tag", tag);
        scope.set_tag("stackoverflow.max_items", max_items);
        scope.set_tag("stackoverflow.has_key", !key.is_none());
        scope.set_tag("interval", interval.as_secs());
        scope.set_tag("discord.url", discord_url.as_str());
    });

    let mut offset: Option<Duration> = None;
    let mut iterations: i32 = 0;
    let stackoverflow_client = reqwest::Client::new();

    loop {
        let now = SystemTime::now();
        let from = match offset {
            // From the last run's 'end' timestamp
            Some(b) => b,
            // From now
            None => now.duration_since(SystemTime::UNIX_EPOCH)?
            // None => now.checked_sub(Duration::from_secs(10000000)).unwrap().duration_since(SystemTime::UNIX_EPOCH)?
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

        match stackoverflow_client.get(&stackoverflow_url).send().await {
            Ok(response) => {
                let status = response.status();
                println!("Stack Overflow Status: {}", status);
                if status == 200 {
                    match response.json().await {
                        Ok(r) => {
                            let response: stackoverflow::Response = r;
                            println!("Response: {:#?}", response);
                            handle_response(&webhook, response).await?;
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

async fn handle_response(
    webhook: &Webhook,
    response: stackoverflow::Response) -> Result<(), Box<dyn std::error::Error>> {
    for item in response.items.iter() {
        println!("item: {:#?}", item);

        sentry::add_breadcrumb(sentry::Breadcrumb {
            category: Some("stackoverflow".into()),
            message: Some(format!("Processing question {}", item.title)),
            ..Default::default()
        });

        let response = webhook.send(|message| { message
            .content("New Question From Stack Overflow.")
            .embed(|embed| embed
                .title(item.title.as_str())
                .field("link", item.link.as_str(), true)
                .image(item.owner.profile_image.as_str(), None, None, None)
                .author(item.owner.display_name.as_str(), 
                    item.owner.link.as_str(),
                    Some(item.owner.profile_image.clone()),
                    None)
            )}).await;

        if let Err(e) = response {
            sentry::capture_message(
                format!("Call to Discord failed with status {} and no body.", e).as_str(), 
                sentry::Level::Error);
        } else {
            println!("Successfully called Discord webhook.");
        }
    }
    Ok(())
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

    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("stackoverflow".into()),
        ty: "url".into(),
        message: Some(url.clone()),
        ..Default::default()
    });

    if let Some(k) = key {
        format!("{}&key={}", url, k)
    } else {
        url
    }
}

mod stackoverflow {
    use super::*;

    #[derive(Deserialize, Debug)]
    pub struct Response {
        pub has_more: bool,
        pub quota_max: u32,
        pub quota_remaining: u32,
        pub items: Vec<Question>,
    }
    
    #[derive(Deserialize, Debug)]
    pub struct Question {
        pub title: String,
        pub link: String,
        pub score: i32,
        pub question_id: u64,
        pub creation_date: u64,
        pub owner: Owner,
        pub tags: Vec<String>,
        pub is_answered: bool,
        pub view_count: u64,
    }
    
    #[derive(Deserialize, Debug)]
    pub struct Owner {
        pub reputation: u64,
        pub user_id: u64,
        pub user_type: String,
        pub accept_rate: Option<u32>,
        pub profile_image: String,
        pub display_name: String,
        pub link: String,
    }
}
