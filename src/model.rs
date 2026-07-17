use serde::{Deserialize, Serialize};

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BusinessModel {
    pub name: String,
    pub level: i64,
    pub reward: i64,
    pub price: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct BusinessCatalogModel {
    pub name: &'static str,
    pub reward: i64,
    pub price: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BetHistoryModel {
    pub minigame: String,
    pub value: i64,
    pub result: String,
    pub datetime: i64,
}

pub static BUSINESSES: &[BusinessCatalogModel] = &[
    BusinessCatalogModel {
        name: "Mercado",
        reward: 1,
        price: 100,
    },
    BusinessCatalogModel {
        name: "Padaria",
        reward: 2,
        price: 700,
    },
    BusinessCatalogModel {
        name: "Farmacia",
        reward: 4,
        price: 3200,
    },
];

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserModel {
    pub _id: String,
    pub coins: i64,
    pub last_reward: i64,
    pub businesses: Vec<BusinessModel>,
    #[serde(default)]
    pub highlow_streak: i64,
    #[serde(default)]
    pub bets: Vec<BetHistoryModel>,
    #[serde(default)]
    pub total_won: i64,
    #[serde(default)]
    pub total_lost: i64,
    #[serde(default)]
    pub wins: i64,
    #[serde(default)]
    pub losses: i64,
}
