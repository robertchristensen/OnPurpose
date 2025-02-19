pub(crate) mod base_data;
pub(crate) mod calculated_data;
mod change_routine;
pub(crate) mod display;
pub(crate) mod menu;
pub(crate) mod new_item;
mod node;
mod surrealdb_layer;
pub(crate) mod systems;

use tokio::sync::mpsc;

use crate::{
    menu::bullet_list_menu::present_normal_bullet_list_menu,
    surrealdb_layer::data_storage_start_and_run,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    const CARGO_PKG_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

    println!("Welcome to On-Purpose: Time Management Rethought");
    println!("Version {}", CARGO_PKG_VERSION.unwrap_or("UNKNOWN"));

    let commands_in_flight_limit = 20;
    let (send_to_data_storage_layer_tx, have_data_storage_layer_use_to_receive_rx) =
        mpsc::channel(commands_in_flight_limit);

    let data_storage_join_handle = tokio::spawn(async move {
        data_storage_start_and_run(
            have_data_storage_layer_use_to_receive_rx,
            "file://c:/.on_purpose.db", //TODO: Get a default file location that works for both Linux and Windows
        )
        .await
    });

    loop {
        match present_normal_bullet_list_menu(&send_to_data_storage_layer_tx).await {
            Result::Ok(..) => (),
            Result::Err(..) => break,
        };

        if data_storage_join_handle.is_finished() {
            println!("Data Storage Layer closed early, unexpectedly");
        }
    }

    drop(send_to_data_storage_layer_tx);

    print!("Waiting for data storage layer to exit...");
    data_storage_join_handle.await.unwrap();
    println!("Done");

    Ok(())
}
