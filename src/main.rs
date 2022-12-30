use async_recursion::async_recursion;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Error;

#[derive(Deserialize, Serialize, Debug)]
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

#[derive(Deserialize, Serialize, Debug)]
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

#[derive(Deserialize, Serialize, Debug)]
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

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
struct Shop {
    name: String,
    myshopify_domain: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct ResultData {
    count: usize,
    total_paid: f32,
    data: Vec<Edges>,
}

#[derive(Deserialize, Serialize, Debug)]
struct Apps {
    id: String,
    app_name: String,
    count: u32,
    total_paid: f32,
    data: Vec<Edges>,
}

#[derive(Serialize, Debug)]
struct RequestBody {
    query: String,
    variables: Variables,
}

#[derive(Serialize, Debug)]
#[serde(rename_all(serialize = "camelCase"))]
struct Variables {
    cursor: String,
    created_at_min: String,
    created_at_max: String,
}

impl RequestBody {
    fn new(cursor: &str, created_at_min: &str, created_at_max: &str) -> Self {
        let variables = Variables {
            cursor: cursor.to_string(),
            created_at_min: created_at_min.to_string(),
            created_at_max: created_at_max.to_string(),
        };
        RequestBody {
            query: r#"
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
                "#.to_string(),
            variables,
        }
    }
}

#[async_recursion]
async fn send_post_request(
    url: &str,
    access_token: &str,
    cursor: &str,
    created_at_min: &str,
    created_at_max: &str,
) -> Result<Vec<Edges>, Error> {
    let body = RequestBody::new(cursor, created_at_min, created_at_max);
    let client = reqwest::Client::new();
    let response_json = client
        .post(url)
        .json(&body)
        .header("X-Shopify-Access-Token", access_token)
        .send()
        .await
        .unwrap()
        .json::<Query>()
        .await
        .unwrap();

    let data = response_json.data.transactions;
    let has_next_page = data.page_info.has_next_page;
    let mut transactions = data.edges;
    let end_cursor = &transactions.last().unwrap().cursor;

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
        transactions.extend(new_transactions);
        return Ok(transactions);
    }
    Ok(transactions)
}

#[tokio::main]
async fn main() -> reqwest::Result<()> {
    dotenv::dotenv().ok();

    let year = 2022;
    let month = 11;

    let dir = String::from("output/") + &year.to_string() + "-" + &month.to_string();
    let created_dir_date = std::fs::create_dir_all(&dir);

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

    let total = ResultData {
        count: transactions.len(),
        total_paid: transactions.iter().fold(0.0, |sum, x| {
            sum + x.node.net_amount.amount.parse::<f32>().unwrap()
        }),
        data: transactions,
    };

    // 合計値のファイルを出力
    let json = serde_json::to_string_pretty(&total).unwrap();
    std::fs::write(String::from(&dir) + "/合計.json", json).unwrap();

    let apps_list = total
        .data
        .into_iter()
        .fold(Vec::new(), |mut result: Vec<Apps>, current| {
            let exists = result.iter_mut().find(|f| f.id == current.node.app.id);

            match exists {
                Some(app) => {
                    let amount = current.node.net_amount.amount.clone();
                    app.data.push(current);
                    app.count += 1;
                    app.total_paid += amount.parse::<f32>().unwrap();
                    result
                }
                None => {
                    let id = current.node.app.id.clone();
                    let app_name = current.node.app.name.clone();
                    let app = Apps {
                        id,
                        app_name,
                        total_paid: current.node.net_amount.amount.parse::<f32>().unwrap(),
                        data: vec![current],
                        count: 1,
                    };
                    result.push(app);
                    result
                }
            }
        });

    // アプリごとの出力
    apps_list.iter().for_each(|app| {
        let json = serde_json::to_string_pretty(app).unwrap();
        let path = format!("{}/{}.json", &dir, app.app_name);
        println!("path:{}", &path);
        std::fs::write(&path, json).unwrap();
    });

    println!("完了");
    Ok(())
}
