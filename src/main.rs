use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use std::convert::Infallible;
use std::convert::TryInto;
use std::net::IpAddr;
use std::time::Duration;
use warp::Filter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnedRecord {
    pub duration: Duration,
    pub name: String,
    pub file: String,
    pub line: u32,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    #[cfg(feature = "dotenv")]
    dotenv::dotenv().ok();

    pretty_env_logger::init();

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:admin@localhost:5432/caolo-profile".to_owned());

    let db_pool = PgPool::builder().max_size(8).build(&db_url).await.unwrap();

    let host = std::env::var("HOST")
        .ok()
        .and_then(|host| {
            host.parse()
                .map_err(|e| {
                    log::error!("Failed to parse host {:?}", e);
                })
                .ok()
        })
        .unwrap_or_else(|| IpAddr::from([127, 0, 0, 1]));
    let port = std::env::var("PORT")
        .map_err(anyhow::Error::new)
        .and_then(|port| port.parse().map_err(anyhow::Error::new))
        .unwrap_or_else(|err| {
            eprintln!("Failed to parse port number: {}", err);
            6660
        });

    let db_pool = warp::any().map(move || db_pool.clone());

    let health = warp::get().and(warp::path("health")).map(|| warp::reply());

    let push_records = warp::post()
        .and(warp::path("push-records"))
        .and(warp::filters::body::json())
        .and(db_pool)
        .and_then(|payload: Vec<OwnedRecord>, db: PgPool| async move {
            let mut tx = db.begin().await.unwrap();

            for row in payload {
                let duration: i64 = row
                    .duration
                    .as_nanos()
                    .try_into()
                    .expect("Failed to convert duration microseconds to 8 byte value");
                sqlx::query!(
                    "
                    INSERT INTO record (duration_ns, name, file, line)
                    VALUES ($1, $2, $3, $4); ",
                    duration,
                    row.name,
                    row.file,
                    row.line as i32
                )
                .execute(&mut tx)
                .await
                .unwrap();
            }

            tx.commit().await.unwrap();

            let resp = warp::reply();
            let resp = warp::reply::with_status(resp, warp::http::StatusCode::NO_CONTENT);
            Ok::<_, Infallible>(resp)
        });

    let api = health.or(push_records);
    let api = api.with(warp::log("cao_profile_collector-router"));

    warp::serve(api).run((host, port)).await;
    Ok(())
}
