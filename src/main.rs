use async_recursion::async_recursion;
use chrono::prelude::*;
use serde_derive::{Deserialize, Serialize};
use serde_json::{json, Error, Value};

#[derive(Deserialize, Debug)]
struct Config {
    access_token: String,
    api_url: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
struct Query {
    data: Data,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
struct Data {
    transactions: Transactions,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
struct Transactions {
    edges: Vec<Edges>,
    page_info: PageInfo,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all(deserialize = "camelCase"))]
struct Edges {
    cursor: String,
    node: Node,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
struct PageInfo {
    has_next_page: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all(deserialize = "camelCase"))]
struct Node {
    id: String,
    created_at: String,
    net_amount: NetAmount,
    app: App,
    shop: Shop,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all(deserialize = "camelCase"))]
struct NetAmount {
    amount: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all(deserialize = "camelCase"))]
struct App {
    id: String,
    name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all(deserialize = "camelCase"))]
struct Shop {
    name: String,
    myshopify_domain: String,
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
) -> Result<Vec<Edges>, Error> {
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
        .json::<Query>()
        .await
        .unwrap();

    let data = response_json.data.transactions;
    let has_next_page = data.page_info.has_next_page;
    let transactions = data.edges.clone();
    let end_cursor = &data.edges.last().unwrap().cursor;

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

    let year = 2022;
    let month = 11;

    let dir = String::from("output/") + &year.to_string() + "-" + &month.to_string();
    let created_dir_date = std::fs::create_dir_all(dir);

    match created_dir_date {
        Ok(()) => println!("フォルダを作成しました。"),
        Err(e) => println!("フォルダ作成エラー: {}", e),
    }

    // 環境変数取得
    let config = match envy::from_env::<Config>() {
        Ok(val) => val,
        Err(err) => {
            println!("{}", err);
            std::process::exit(1);
        }
    };

    //  取得するデータの日時の範囲
    let created_at_min = Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0).unwrap();
    let created_at_max = Utc.with_ymd_and_hms(year, month + 1, 1, 0, 0, 0).unwrap();

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

    let data_to_json = serde_json::to_string_pretty(&transactions).unwrap();
    std::fs::write("output/hello.json", data_to_json).unwrap();

    println!("配列の数：{}", transactions.len());
    println!("完了");
    Ok(())
}
