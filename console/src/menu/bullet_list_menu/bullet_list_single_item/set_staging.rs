use async_recursion::async_recursion;
use tokio::sync::mpsc::Sender;

use crate::{
    menu::staging_query::{mentally_resident_query, on_deck_query},
    node::item_node::ItemNode,
    surrealdb_layer::{
        surreal_item::{Responsibility, Staging},
        DataLayerCommands,
    },
};
use inquire::{InquireError, Select};
use std::fmt::Display;

enum StagingMenuSelection {
    NotSet,
    MentallyResident,
    OnDeck,
    Intension,
    Released,
    MakeItemReactive,
}

impl Display for StagingMenuSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StagingMenuSelection::NotSet => write!(f, "Not Set"),
            StagingMenuSelection::MentallyResident => write!(f, "Mentally Resident"),
            StagingMenuSelection::OnDeck => write!(f, "On Deck"),
            StagingMenuSelection::Intension => write!(f, "Intension"),
            StagingMenuSelection::Released => write!(f, "Released"),
            StagingMenuSelection::MakeItemReactive => write!(f, "Make Item Reactive"),
        }
    }
}

impl StagingMenuSelection {
    fn make_list() -> Vec<Self> {
        vec![
            StagingMenuSelection::OnDeck,
            StagingMenuSelection::MentallyResident,
            StagingMenuSelection::Intension,
            StagingMenuSelection::Released,
            StagingMenuSelection::NotSet,
            StagingMenuSelection::MakeItemReactive,
        ]
    }
}

#[async_recursion]
pub(crate) async fn present_set_staging_menu(
    selected: &ItemNode<'_>,
    send_to_data_storage_layer: &Sender<DataLayerCommands>,
) {
    let list = StagingMenuSelection::make_list();

    let selection = Select::new("", list).prompt().unwrap();
    let staging = match selection {
        StagingMenuSelection::NotSet => Staging::NotSet,
        StagingMenuSelection::MentallyResident => {
            let result = mentally_resident_query().await;
            match result {
                Ok(mentally_resident) => mentally_resident,
                Err(InquireError::OperationCanceled) => {
                    return present_set_staging_menu(selected, send_to_data_storage_layer).await
                }
                Err(err) => todo!("{:?}", err),
            }
        }
        StagingMenuSelection::OnDeck => {
            let result = on_deck_query().await;
            match result {
                Ok(staging) => staging,
                Err(InquireError::OperationCanceled) => {
                    return present_set_staging_menu(selected, send_to_data_storage_layer).await
                }
                Err(err) => todo!("{:?}", err),
            }
        }
        StagingMenuSelection::Intension => Staging::Intension,
        StagingMenuSelection::Released => Staging::Released,
        StagingMenuSelection::MakeItemReactive => {
            return send_to_data_storage_layer
                .send(DataLayerCommands::UpdateItemResponsibility(
                    selected.get_surreal_item().clone(),
                    Responsibility::ReactiveBeAvailableToAct,
                ))
                .await
                .unwrap();
        }
    };

    send_to_data_storage_layer
        .send(DataLayerCommands::UpdateItemStaging(
            selected.get_surreal_item().clone(),
            staging,
        ))
        .await
        .unwrap();
}