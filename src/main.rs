use chrono::{TimeZone, Utc};
use clap::Parser;
use serde::{Deserialize, Serialize};

mod api_schema;
mod request_api;

#[derive(Parser)]
#[command(version = "1.0", about = "Get the count of active users using the Shopify Partner API.", long_about = None)]
struct Cli {
    #[arg(short, long, help = "年を指定してください", value_parser(clap::value_parser!(i32).range(2000..=2100)))]
    year: i32,

    #[arg(short, long, help = "月を指定してください", value_parser(clap::value_parser!(u32).range(1..=12)))]
    month: u32,
}

#[derive(Deserialize, Serialize, Debug)]
struct Config {
    access_token: String,
    api_url: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let year = cli.year;
    let month = cli.month;
    let end_year = if month == 12 { year + 1 } else { year };
    let end_month = if month == 12 { 1 } else { month + 1 };

    println!(
        "target: {}-{:02} ~ {}-{:02}",
        year, month, end_year, end_month
    );
    dotenv::dotenv().ok();

    let dir = format!("output/{}-{:02}", &year, &month);
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
    let created_at_max = Utc
        .with_ymd_and_hms(end_year, end_month, 1, 0, 0, 0)
        .unwrap();

    // 日付範囲内のデータを全件取得
    let transactions = request_api::get(
        &config.api_url,
        &config.access_token,
        "",
        &created_at_min.to_rfc3339(),
        &created_at_max.to_rfc3339(),
    )
    .await
    .unwrap();

    let total = api_schema::ResultData {
        count: transactions.len(),
        total_paid: transactions.iter().fold(0.0, |sum, x| {
            sum + x.node.net_amount.amount.parse::<f32>().unwrap()
        }),
        data: transactions,
    };

    // 合計値のファイルを出力
    let json = serde_json::to_string_pretty(&total).unwrap();
    std::fs::write(String::from(&dir) + "/合計.json", json).unwrap();

    let apps_list =
        total
            .data
            .into_iter()
            .fold(Vec::new(), |mut result: Vec<api_schema::Apps>, current| {
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
                        let app = api_schema::Apps {
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
}
