use std::cmp::Ordering;

use async_recursion::async_recursion;
use chrono::Utc;
use inquire::{InquireError, Select};
use tokio::sync::mpsc::Sender;

use crate::{
    base_data::{
        item::{Item, ItemVecExtensions},
        BaseData,
    },
    display::display_item_node::DisplayItemNode,
    menu::{
        bullet_list_menu::bullet_list_single_item::ItemTypeSelection,
        select_higher_priority_than_this::select_higher_priority_than_this,
    },
    node::item_node::ItemNode,
    surrealdb_layer::{surreal_tables::SurrealTables, DataLayerCommands},
};

pub(crate) async fn parent_to_a_motivation(
    parent_this: &Item<'_>,
    send_to_data_storage_layer: &Sender<DataLayerCommands>,
) -> Result<(), ()> {
    let surreal_tables = SurrealTables::new(send_to_data_storage_layer)
        .await
        .unwrap();
    let now = Utc::now();
    let base_data = BaseData::new_from_surreal_tables(surreal_tables, now);
    let active_items = base_data.get_active_items();
    let items = active_items
        .filter_just_motivations()
        .map(|x| {
            ItemNode::new(
                x,
                base_data.get_coverings(),
                base_data.get_active_snoozed(),
                active_items,
            )
        })
        .collect::<Vec<_>>();
    let list = items.iter().map(DisplayItemNode::new).collect::<Vec<_>>();

    let selection = Select::new("Select from the below list|", list).prompt();
    match selection {
        Ok(parent) => {
            let parent = parent.get_item_node();
            let parent_smaller = parent.get_smaller();
            let higher_priority_than_this = if !parent_smaller.is_empty() {
                let children = parent_smaller
                    .iter()
                    .map(|x| x.get_item())
                    .collect::<Vec<_>>();
                select_higher_priority_than_this(&children)
            } else {
                None
            };
            send_to_data_storage_layer
                .send(DataLayerCommands::ParentItemWithExistingItem {
                    child: parent_this.get_surreal_record_id().clone(),
                    parent: parent.get_surreal_record_id().clone(),
                    higher_priority_than_this,
                })
                .await
                .unwrap();
            Ok(())
        }
        Err(InquireError::OperationCanceled | InquireError::InvalidConfiguration(_)) => {
            parent_to_a_motivation_new_motivation(parent_this, send_to_data_storage_layer).await
        }
        Err(err) => {
            todo!("Error: {:?}", err);
        }
    }
}

pub(crate) async fn parent_to_a_goal_or_motivation(
    parent_this: &Item<'_>,
    send_to_data_storage_layer: &Sender<DataLayerCommands>,
) -> Result<(), ()> {
    let surreal_tables = SurrealTables::new(send_to_data_storage_layer)
        .await
        .unwrap();
    let now = Utc::now();
    let base_data = BaseData::new_from_surreal_tables(surreal_tables, now);
    let active_items = base_data.get_active_items();
    let mut list = active_items
        .iter()
        .filter(|x| x.is_type_goal() || x.is_type_motivation())
        .map(|item| {
            ItemNode::new(
                item,
                base_data.get_coverings(),
                base_data.get_active_snoozed(),
                active_items,
            )
        })
        //Collect the ItemNodes because they need a place to be so they don't go out of scope as DisplayItemNode
        //only takes a reference.
        .collect::<Vec<_>>();

    list.sort_by(|a, b| {
        //Motivational items first
        if a.is_type_motivation() && !b.is_type_motivation() {
            Ordering::Less
        } else if !a.is_type_motivation() && b.is_type_motivation() {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    });

    let list = list.iter().map(DisplayItemNode::new).collect::<Vec<_>>();

    let selection = Select::new("Select from the below list|", list).prompt();
    match selection {
        Ok(parent) => {
            let parent: &ItemNode<'_> = parent.get_item_node();

            let higher_priority_than_this = if parent.has_active_children() {
                let items = parent
                    .get_smaller()
                    .iter()
                    .map(|x| x.get_item())
                    .collect::<Vec<_>>();
                select_higher_priority_than_this(&items)
            } else {
                None
            };
            send_to_data_storage_layer
                .send(DataLayerCommands::ParentItemWithExistingItem {
                    child: parent_this.get_surreal_record_id().clone(),
                    parent: parent.get_surreal_record_id().clone(),
                    higher_priority_than_this,
                })
                .await
                .unwrap();
            Ok(())
        }
        Err(InquireError::OperationCanceled) => {
            parent_to_a_goal_or_motivation_new_goal_or_motivation(
                parent_this,
                send_to_data_storage_layer,
            )
            .await
        }
        Err(InquireError::OperationInterrupted) => Err(()),
        Err(err) => {
            todo!("Error: {:?}", err);
        }
    }
}

#[async_recursion]
async fn parent_to_a_motivation_new_motivation(
    parent_this: &Item<'_>,
    send_to_data_storage_layer: &Sender<DataLayerCommands>,
) -> Result<(), ()> {
    let list = ItemTypeSelection::create_list_just_motivations();
    let selection = Select::new("Select from the below list|", list).prompt();
    match selection {
        Ok(ItemTypeSelection::NormalHelp) => {
            ItemTypeSelection::print_normal_help();
            parent_to_a_motivation_new_motivation(parent_this, send_to_data_storage_layer).await
        }
        Ok(ItemTypeSelection::ResponsiveHelp) => {
            ItemTypeSelection::print_responsive_help();
            parent_to_a_motivation_new_motivation(parent_this, send_to_data_storage_layer).await
        }
        Ok(item_type_selection) => {
            let new_item = item_type_selection.create_new_item_prompt_user_for_summary();
            send_to_data_storage_layer
                .send(DataLayerCommands::ParentNewItemWithAnExistingChildItem {
                    child: parent_this.get_surreal_record_id().clone(),
                    parent_new_item: new_item,
                })
                .await
                .unwrap();
            Ok(())
        }
        Err(InquireError::OperationCanceled) => {
            todo!("I need to go back to what first called this");
        }
        Err(InquireError::OperationInterrupted) => Err(()),
        Err(err) => {
            todo!("Error: {:?}", err);
        }
    }
}

#[async_recursion]
async fn parent_to_a_goal_or_motivation_new_goal_or_motivation(
    parent_this: &Item<'_>,
    send_to_data_storage_layer: &Sender<DataLayerCommands>,
) -> Result<(), ()> {
    let list = ItemTypeSelection::create_list_goals_and_motivations();
    let selection = Select::new("Select from the below list|", list).prompt();
    match selection {
        Ok(ItemTypeSelection::NormalHelp) => {
            ItemTypeSelection::print_normal_help();
            parent_to_a_goal_or_motivation_new_goal_or_motivation(
                parent_this,
                send_to_data_storage_layer,
            )
            .await
        }
        Ok(ItemTypeSelection::ResponsiveHelp) => {
            ItemTypeSelection::print_responsive_help();
            parent_to_a_goal_or_motivation_new_goal_or_motivation(
                parent_this,
                send_to_data_storage_layer,
            )
            .await
        }
        Ok(item_type_selection) => {
            let new_item = item_type_selection.create_new_item_prompt_user_for_summary();
            send_to_data_storage_layer
                .send(DataLayerCommands::ParentNewItemWithAnExistingChildItem {
                    child: parent_this.get_surreal_record_id().clone(),
                    parent_new_item: new_item,
                })
                .await
                .unwrap();
            Ok(())
        }
        Err(InquireError::OperationCanceled) => {
            todo!("I need to go back to what first called this");
        }
        Err(InquireError::OperationInterrupted) => Err(()),
        Err(err) => {
            todo!("Error: {:?}", err);
        }
    }
}
