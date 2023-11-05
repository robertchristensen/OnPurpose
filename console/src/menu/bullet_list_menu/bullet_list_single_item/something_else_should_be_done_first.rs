use inquire::{InquireError, Select};
use tokio::sync::mpsc::Sender;

use crate::{
    base_data::{item::Item, BaseData},
    display::display_item::DisplayItem,
    surrealdb_layer::DataLayerCommands,
};

use super::ItemTypeSelection;

pub(crate) async fn something_else_should_be_done_first(
    unable_to_do: &Item<'_>,
    send_to_data_storage_layer: &Sender<DataLayerCommands>,
) {
    let surreal_tables = DataLayerCommands::get_raw_data(send_to_data_storage_layer)
        .await
        .unwrap();
    let base_data = BaseData::new_from_surreal_tables(surreal_tables);
    let list = base_data
        .get_active_items()
        .iter()
        .copied()
        .map(DisplayItem::new)
        .collect::<Vec<_>>();
    let selection = Select::new("", list).prompt();
    match selection {
        Ok(should_be_done_first) => send_to_data_storage_layer
            .send(DataLayerCommands::CoverItemWithAnExistingItem {
                item_to_be_covered: unable_to_do.get_surreal_item().clone(),
                item_that_should_do_the_covering: should_be_done_first.get_surreal_item().clone(),
            })
            .await
            .unwrap(),
        Err(InquireError::OperationCanceled | InquireError::InvalidConfiguration(_)) => {
            something_else_should_be_done_first_new_item(unable_to_do, send_to_data_storage_layer)
                .await;
        }
        Err(err) => {
            todo!("Error: {:?}", err);
        }
    }
}

pub(crate) async fn something_else_should_be_done_first_new_item(
    unable_to_do: &Item<'_>,
    send_to_data_storage_layer: &Sender<DataLayerCommands>,
) {
    let list = ItemTypeSelection::create_list();
    let selection = Select::new("", list).prompt();
    match selection {
        Ok(selection) => {
            let new_item = selection.create_new_item_prompt_user_for_summary();
            send_to_data_storage_layer
                .send(DataLayerCommands::CoverItemWithANewItem {
                    cover_this: unable_to_do.get_surreal_item().clone(),
                    cover_with: new_item,
                })
                .await
                .unwrap();
        }
        Err(_) => todo!(),
    }
}
