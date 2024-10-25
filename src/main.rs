use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;
use zero2prod::{configuration::get_configuration, startup::run};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let configuration = get_configuration().expect("Failed to read configuration.");
    let connection_string = configuration.database.connection_string();

    let db_pool = Arc::new(
        PgPoolOptions::new()
            .max_connections(5)
            .connect(&connection_string)
            .await
            .expect("Failed to connect to Postgres"),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:9000").await?;
    run(listener, db_pool).await
}
