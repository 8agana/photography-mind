use crate::config::Config;
use anyhow::Result;
use surrealdb::{
    Surreal,
    engine::remote::ws::{Client, Ws},
    opt::auth::Root,
};

pub async fn connect_db(cfg: &Config) -> Result<Surreal<Client>> {
    // Surreal expects host:port without scheme for Ws; strip ws:// or wss:// if present.
    let addr = cfg
        .db_url
        .trim_start_matches("ws://")
        .trim_start_matches("wss://")
        .to_string();

    let db = Surreal::new::<Ws>(&addr).await?;
    db.signin(Root {
        username: &cfg.db_user,
        password: &cfg.db_pass,
    })
    .await?;
    db.use_ns(&cfg.db_namespace).use_db(&cfg.db_name).await?;
    Ok(db)
}

pub async fn healthcheck(db: &Surreal<Client>) -> Result<bool> {
    // simple ping via info query
    let mut res = db.query("RETURN 1;").await?;
    let val: Option<i32> = res.take(0)?;
    Ok(val == Some(1))
}
