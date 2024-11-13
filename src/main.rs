use std::fmt::{self, Display};

use tokio::task::JoinError;
use zero2prod::{
    configuration::get_configuration,
    issue_delivery_worker::run_worker_until_stopped,
    startup::{run_until_stopped, AppState},
    telemetry::init,
    Result,
};

#[tokio::main]
async fn main() -> Result<()> {
    let configuration = get_configuration().expect("Failed to read configuration.");
    init(&configuration.logger);
    let app_state = AppState::build(&configuration).await;
    let application_task = tokio::spawn(run_until_stopped(app_state, configuration.clone()));
    let worker_task = tokio::spawn(run_worker_until_stopped(configuration.clone()));
    tokio::select! {
        o = application_task => report_exit("API", o),
        o = worker_task =>  report_exit("Background worker", o),
    };
    Ok(())
}

fn report_exit(
    task_name: &str,
    outcome: std::result::Result<std::result::Result<(), impl fmt::Debug + Display>, JoinError>,
) {
    match outcome {
        Ok(Ok(())) => {
            tracing::info!("{} has exited", task_name)
        }
        Ok(Err(e)) => {
            tracing::error!("{} failed with error: {}", task_name, e);
        }
        Err(e) => {
            tracing::error!("{} task failed to complete with error{}", task_name, e);
        }
    }
}
