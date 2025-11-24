use crate::config::Config;
use anyhow::Result;
use surrealdb::{engine::remote::ws::{Client, Ws}, opt::auth::Root, Surreal};

pub async fn connect_db(cfg: &Config) -> Result<Surreal<Client>> {
    let db = Surreal::new::<Ws>(&cfg.db_url).await?;
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
