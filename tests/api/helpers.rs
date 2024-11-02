use once_cell::sync::Lazy;
use uuid::Uuid;
use zero2prod::{
    configuration::get_configuration,
    startup::{configuration_database, AppState},
    telemetry::init_tracing,
};

static TRACING: Lazy<()> = Lazy::new(|| {
    init_tracing();
});

pub async fn spawn_app() -> AppState {
    Lazy::force(&TRACING);
    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.database.database_name = Uuid::new_v4().to_string();
    configuration_database(&configuration.database).await;
    AppState::build(&configuration).await
}
