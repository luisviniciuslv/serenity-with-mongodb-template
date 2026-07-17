use crate::model::{BetHistoryModel, UserModel, BUSINESSES};

use lazy_static::lazy_static;
use mongodb::bson::from_document;
use mongodb::bson::to_bson;
use mongodb::bson::{doc, Document};
use mongodb::error::Result;
use mongodb::{options::ClientOptions, Client, Collection};
use serenity::futures::TryStreamExt;
use std::sync::Mutex;

use std::time::{SystemTime, UNIX_EPOCH};

const REWARD_INTERVAL_SECONDS: i64 = 2;
const BASE_REWARD_PER_INTERVAL: i64 = 1;
const MAX_REWARD_PER_COLLECTION: i64 = 1000;
const CAP_BONUS_PER_BUSINESS_LEVEL: i64 = 500;

lazy_static! {
    static ref DB_CONNECTION: Mutex<Option<Client>> = Mutex::new(None);
    static ref USER_COLLECTION: Mutex<Option<Collection<Document>>> = Mutex::new(None);
}

pub async fn init() -> Result<()> {
    dotenv::dotenv().expect("Failed to load .env file");

    let mongodb_uri: String = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set.");
    let database_name: String =
        std::env::var("MONGO_INITDB_DATABASE").expect("MONGO_INITDB_DATABASE must be set.");
    let user_collection_name: String =
        std::env::var("MONGODB_USERS_COLLECTION").expect("MONGODB_USERS_COLLECTION must be set.");

    let mut client_options = ClientOptions::parse(mongodb_uri).await?;
    client_options.app_name = Some(database_name.to_string());

    let client = Client::with_options(client_options)?;
    let database = client.database(&database_name);

    *DB_CONNECTION.lock().unwrap() = Some(client);
    *USER_COLLECTION.lock().unwrap() = Some(database.collection::<Document>(&user_collection_name));

    println!("✅ Database connected successfully");
    Ok(())
}

pub async fn update_user(user: &UserModel) -> Result<()> {
    let user_collection: Collection<Document> = USER_COLLECTION.lock().unwrap().clone().unwrap();
    let bson_user = to_bson(user)?;
    let document: mongodb::bson::Document = bson_user.as_document().unwrap().clone();
    user_collection
        .update_one(doc! {"_id": user.clone()._id}, doc! {"$set": document})
        .await?;
    Ok(())
}

pub async fn create_user(user_id: &str) -> Result<UserModel> {
    let user_collection: Collection<Document> = USER_COLLECTION.lock().unwrap().clone().unwrap();
    let user = user_collection.find_one(doc! {"_id": user_id}).await?;
    if user.is_none() {
        let unix_time_i64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64;

        let user = UserModel {
            _id: user_id.to_string(),
            coins: 100,
            last_reward: unix_time_i64,
            businesses: Vec::new(),
            highlow_streak: 0,
            bets: Vec::new(),
        };

        let bson_user = to_bson(&user)?;
        let document: mongodb::bson::Document = bson_user.as_document().unwrap().clone();
        user_collection.insert_one(document).await?;
        return Ok(user);
    } else {
        return Err(mongodb::error::Error::from(std::io::Error::new(
            std::io::ErrorKind::Other,
            "User already exists",
        )));
    }
}

pub async fn get_user(user_id: &str) -> Result<UserModel> {
    let user_collection: Collection<Document> = USER_COLLECTION.lock().unwrap().clone().unwrap();

    let user_doc = user_collection.find_one(doc! {"_id": user_id}).await?;
    if let Some(doc) = user_doc {
        // Convert the Document back to UserModel
        let user: UserModel = from_document(doc)?;
        Ok(user)
    } else {
        create_user(user_id).await
    }
}

pub async fn update_coins(user_id: &str, coins: i64) -> Result<UserModel> {
    let user_collection: Collection<Document> = USER_COLLECTION.lock().unwrap().clone().unwrap();
    user_collection
        .update_one(doc! {"_id": user_id}, doc! {"$inc": doc! {"coins": coins}})
        .await?;
    get_user(user_id).await
}

pub async fn set_highlow_streak(user_id: &str, streak: i64) -> Result<UserModel> {
    let user_collection: Collection<Document> = USER_COLLECTION.lock().unwrap().clone().unwrap();
    user_collection
        .update_one(
            doc! {"_id": user_id},
            doc! {"$set": doc! {"highlow_streak": streak}},
        )
        .await?;
    get_user(user_id).await
}

pub async fn record_bet(user_id: &str, minigame: &str, value: i64, won: bool) -> Result<UserModel> {
    let user_collection: Collection<Document> = USER_COLLECTION.lock().unwrap().clone().unwrap();
    let bet = BetHistoryModel {
        minigame: minigame.to_string(),
        value,
        result: if won {
            "vitoria".to_string()
        } else {
            "derrota".to_string()
        },
        datetime: get_current_timestamp(),
    };

    user_collection
        .update_one(
            doc! {"_id": user_id},
            doc! {"$push": doc! {"bets": mongodb::bson::to_document(&bet)?}},
        )
        .await?;

    get_user(user_id).await
}

pub async fn clear_users_collection() -> Result<u64> {
    let user_collection: Collection<Document> = USER_COLLECTION.lock().unwrap().clone().unwrap();
    let result = user_collection.delete_many(doc! {}).await?;
    Ok(result.deleted_count)
}

pub fn get_current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as i64
}

pub fn get_reward_interval_seconds() -> i64 {
    REWARD_INTERVAL_SECONDS
}

pub fn get_max_reward_per_collection(user: &UserModel) -> i64 {
    let total_business_levels: i64 = user.businesses.iter().map(|business| business.level).sum();

    MAX_REWARD_PER_COLLECTION + (total_business_levels * CAP_BONUS_PER_BUSINESS_LEVEL)
}

pub fn get_reward_per_interval(user: &UserModel) -> i64 {
    let business_income: i64 = user
        .businesses
        .iter()
        .map(|business| {
            let reward_from_catalog = BUSINESSES
                .iter()
                .find(|catalog| catalog.name == business.name)
                .map(|catalog| catalog.reward)
                .unwrap_or(business.reward);

            reward_from_catalog * business.level
        })
        .sum();

    BASE_REWARD_PER_INTERVAL + business_income
}

pub fn calculate_pending_reward(user: &UserModel, now: i64) -> (i64, i64, bool) {
    let elapsed_seconds = (now - user.last_reward).max(0);
    let completed_intervals = elapsed_seconds / REWARD_INTERVAL_SECONDS;
    let dynamic_cap = get_max_reward_per_collection(user);
    let raw_reward_amount = completed_intervals * get_reward_per_interval(user);
    let was_capped = raw_reward_amount > dynamic_cap;
    let reward_amount = raw_reward_amount.min(dynamic_cap);

    (reward_amount, completed_intervals, was_capped)
}

pub async fn collect_reward(user_id: &str) -> Result<(UserModel, i64, bool)> {
    let user_collection: Collection<Document> = USER_COLLECTION.lock().unwrap().clone().unwrap();
    let mut user = get_user(user_id).await?;
    let now = get_current_timestamp();
    let (reward_amount, completed_intervals, was_capped) = calculate_pending_reward(&user, now);

    if reward_amount > 0 {
        let consumed_seconds = completed_intervals * REWARD_INTERVAL_SECONDS;
        // Use atomic increments to avoid overwriting concurrent game coin updates.
        user_collection
            .update_one(
                doc! {"_id": user_id},
                doc! {
                  "$inc": doc! {
                    "coins": reward_amount,
                    "last_reward": consumed_seconds,
                  }
                },
            )
            .await?;
        user = get_user(user_id).await?;
    }

    Ok((user, reward_amount, was_capped))
}

pub async fn get_all_users() -> Result<Vec<UserModel>> {
    let user_collection: Collection<Document> = USER_COLLECTION.lock().unwrap().clone().unwrap();
    let mut users = Vec::new();

    let mut cursor = user_collection.find(doc! {}).await?;
    while let Some(doc) = cursor.try_next().await? {
        let user: UserModel = from_document(doc)?;
        users.push(user);
    }

    Ok(users)
}
