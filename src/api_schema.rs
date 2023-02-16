use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct Query {
    pub data: Data,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct Data {
    pub transactions: Transactions,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct Transactions {
    pub edges: Vec<Edges>,
    pub page_info: PageInfo,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct Edges {
    pub cursor: String,
    pub node: Node,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct PageInfo {
    pub has_next_page: bool,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct Node {
    pub id: String,
    pub created_at: String,
    pub net_amount: NetAmount,
    pub app: App,
    pub shop: Shop,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct NetAmount {
    pub amount: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct App {
    pub id: String,
    pub name: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct Shop {
    pub name: String,
    pub myshopify_domain: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ResultData {
    pub count: usize,
    pub total_paid: f32,
    pub data: Vec<Edges>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Apps {
    pub id: String,
    pub app_name: String,
    pub count: u32,
    pub total_paid: f32,
    pub data: Vec<Edges>,
}

#[derive(Serialize, Debug)]
pub struct RequestBody {
    pub query: String,
    pub variables: Variables,
}

#[derive(Serialize, Debug)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct Variables {
    pub cursor: String,
    pub created_at_min: String,
    pub created_at_max: String,
}

impl RequestBody {
    pub fn new(cursor: &str, created_at_min: &str, created_at_max: &str) -> Self {
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
