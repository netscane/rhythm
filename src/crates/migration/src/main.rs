use env_logger::Env;
use sea_orm_migration::prelude::*;

#[async_std::main]
async fn main() {
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    cli::run_cli(migration::Migrator).await;
}
