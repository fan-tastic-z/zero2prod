use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;
use zero2prod::{
    configuration::get_configuration, email_client::EmailClient, startup::run,
    telemetry::init_tracing,
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    init_tracing();

    let configuration = get_configuration().expect("Failed to read configuration.");

    let connection_pool = Arc::new(
        PgPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_secs(2))
            .connect_lazy_with(configuration.database.with_db()),
    );
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );

    let sender_email = configuration
        .email_client
        .sender()
        .expect("Invalid sender email address");
    let timeout = configuration.email_client.timeout();
    let email_client = Arc::new(EmailClient::new(
        configuration.email_client.base_url,
        sender_email,
        configuration.email_client.authorization_token,
        timeout,
    ));

    let listener = tokio::net::TcpListener::bind(address).await?;
    run(listener, connection_pool, email_client).await
}
