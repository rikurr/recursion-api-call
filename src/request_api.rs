use crate::api_schema;
use async_recursion::async_recursion;
use serde_json::Error;

#[async_recursion]
pub async fn get(
    url: &str,
    access_token: &str,
    cursor: &str,
    created_at_min: &str,
    created_at_max: &str,
) -> Result<Vec<api_schema::Edges>, Error> {
    let body = api_schema::RequestBody::new(cursor, created_at_min, created_at_max);
    let client = reqwest::Client::new();
    let response_json = client
        .post(url)
        .json(&body)
        .header("X-Shopify-Access-Token", access_token)
        .send()
        .await
        .unwrap()
        .json::<api_schema::Query>()
        .await
        .unwrap();

    let data = response_json.data.transactions;
    let has_next_page = data.page_info.has_next_page;
    let mut transactions = data.edges;
    let end_cursor = &transactions.last().unwrap().cursor;

    if has_next_page {
        let new_transactions = get(
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
