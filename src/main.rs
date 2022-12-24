use async_recursion::async_recursion;
use chrono::prelude::*;
use serde_json::{json, Error, Value};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Config {
    access_token: String,
    api_url: String,
}


fn generate_query(cursor: &str, created_at_min: &str, created_at_max: &str) -> Value {
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
            "createdAtMin": created_at_min,
            "createdAtMax": created_at_max,
        }
    })
}

#[async_recursion]
async fn send_post_request(
    url: &str,
    access_token: &str,
    cursor: &str,
    created_at_min: &str,
    created_at_max: &str,
) -> Result<Vec<Value>, Error> {
    let body = generate_query(cursor, created_at_min, created_at_max);
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

    if has_next_page {
        let new_transactions = send_post_request(
            url,
            access_token,
            end_cursor,
            created_at_min,
            created_at_max,
        )
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

    let config = match envy::from_env::<Config>() {
        Ok(val) => val,
        Err(err) => {
            println!("{}", err);
            std::process::exit(1);
        }
    };

    //  取得するデータの日時の範囲
    let created_at_min = Utc.with_ymd_and_hms(2022, 11, 1, 0, 0, 0).unwrap();
    let created_at_max = Utc.with_ymd_and_hms(2022, 12, 1, 0, 0, 0).unwrap();
    
    // 日付範囲内のデータを全件取得
    let transactions = send_post_request(
        &config.api_url,
        &config.access_token,
        "",
        &created_at_min.to_rfc3339(),
        &created_at_max.to_rfc3339(),
    )
    .await
    .unwrap();

    println!("配列の数：{}", transactions.len());
    println!("完了");
    Ok(())
}
