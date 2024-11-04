use zero2prod::{
    configuration::get_configuration,
    startup::{run_until_stopped, AppState},
    telemetry::init,
    Result,
};

#[tokio::main]
async fn main() -> Result<()> {
    let configuration = get_configuration().expect("Failed to read configuration.");
    init(&configuration.logger);
    let app_state = AppState::build(&configuration).await;
    run_until_stopped(app_state, configuration).await
}
