use async_recursion::async_recursion;
use chrono::prelude::*;
use serde_json::{json, Error, Value};

fn generate_query(cursor: &str) -> Value {
    json!({
        "query": "
            query($cursor: String, $createdAtMin: DateTime, $createdAtMax: DateTime) {
                transactions(types: [APP_SUBSCRIPTION_SALE] after: $cursor, first: 100, createdAtMin: $createdAtMin, createdAtMax: $createdAtMax) {
                    edges {
                        cursor
                        node {
                                id
                                createdAt
                                ... on AppSubscriptionSale {
                                netAmount {
                                    amount
                            }
                            app {
                                    id
                                    name
                            }
                            shop {
                                    name
                                    myshopifyDomain
                                }
                            }
                        }
                        }
                        pageInfo {
                        hasNextPage
                        }
                    }
                }
            ",
        "variables": {
            "cursor": cursor,
            "createdAtMin": "2022-11-28T00:00:00.000000Z",
            "createdAtMax": "2022-12-01T00:00:00.000000Z"
        }
    })
}

#[async_recursion]
async fn send_post_request(
    url: &str,
    access_token: &str,
    cursor: &str,
) -> Result<Vec<Value>, Error> {
    println!("今のデータ{}", cursor);
    let body = generate_query(cursor);
    let client = reqwest::Client::new();
    let response_json = client
        .post(url)
        .json(&body)
        .header(reqwest::header::CONTENT_TYPE, "application/graphql")
        .header("X-Shopify-Access-Token", access_token)
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();

    println!("{:?}", response_json);
    let data = response_json
        .get("data")
        .unwrap()
        .get("transactions")
        .unwrap();
    let has_next_page = data
        .get("pageInfo")
        .unwrap()
        .get("hasNextPage")
        .unwrap()
        .as_bool()
        .unwrap();
    let transactions = data.get("edges").unwrap().as_array().unwrap().clone();
    let end_cursor = data.get("edges").unwrap().as_array().unwrap()[transactions.len() - 1]
        .get("cursor")
        .unwrap()
        .as_str()
        .unwrap();
    println!("何個{}", transactions.len());

    if has_next_page {
        println!("次のデータ{}", end_cursor);
        let new_transactions = send_post_request(url, access_token, end_cursor)
            .await
            .unwrap();
        let actual = [&transactions[..], &new_transactions[..]].concat();
        return Ok(actual);
    }
    Ok(transactions)
}

#[tokio::main]
async fn main() -> reqwest::Result<()> {
    dotenv::dotenv().ok();

    let access_token_key = "ACCESS_TOKEN";
    let api_url_key = "API_URL";

    let url = match std::env::var(api_url_key) {
        Ok(val) => val,
        Err(err) => {
            println!("{}:{}", err, api_url_key);
            std::process::exit(1);
        }
    };
    let access_token = match std::env::var(access_token_key) {
        Ok(val) => val,
        Err(err) => {
            println!("{}:{}", err, access_token_key);
            std::process::exit(1);
        }
    };
    let transactions = send_post_request(&url, &access_token, "").await.unwrap();
    let date: DateTime<Utc> = Utc.with_ymd_and_hms(2022, 12, 1, 0, 0, 0).unwrap();
    println!("{}", date);

    println!("配列の数：{}", transactions.len());
    println!("完了");
    Ok(())
}
