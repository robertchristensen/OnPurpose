use chrono::{DateTime, Local};

use crate::base_data::{Covering, CoveringUntilDateTime, Item, ToDo};

pub struct GrowingNode<'a> {
    pub item: &'a Item<'a>,
    pub larger: Vec<GrowingNode<'a>>,
}

impl<'a> GrowingNode<'a> {
    pub fn create_growing_parents(&self) -> Vec<&'a Item<'a>> {
        let mut result = Vec::default();
        for i in self.larger.iter() {
            result.push(i.item);
            let parents = i.create_growing_parents();
            result.extend(parents.iter());
        }
        result
    }
}

pub struct ToDoNode<'a> {
    pub to_do: &'a ToDo<'a>,
    pub larger: Vec<GrowingNode<'a>>,
}

pub fn create_to_do_nodes<'a>(
    next_steps: &'a [ToDo],
    coverings: &'a [Covering<'a>],
    coverings_until_date_time: &'a [CoveringUntilDateTime<'a>],
    current_date: &DateTime<Local>,
    currently_in_focus_time: bool,
) -> Vec<ToDoNode<'a>> {
    next_steps
        .iter()
        .filter_map(|x| {
            if !x.is_covered(coverings, coverings_until_date_time, current_date)
                && !x.is_finished()
                && x.is_circumstances_met(current_date, currently_in_focus_time)
            {
                Some(create_to_do_node(x, coverings))
            } else {
                None
            }
        })
        .collect()
}

pub fn create_to_do_node<'a>(to_do: &'a ToDo, coverings: &'a [Covering<'a>]) -> ToDoNode<'a> {
    let item: &Item = to_do.into();
    let parents = item.find_parents(coverings);
    let larger = create_growing_nodes(parents, coverings);

    ToDoNode { to_do, larger }
}

pub fn create_growing_nodes<'a>(
    items: Vec<&'a Item<'a>>,
    coverings: &'a [Covering<'a>],
) -> Vec<GrowingNode<'a>> {
    items
        .iter()
        .map(|x| create_growing_node(x, coverings))
        .collect()
}

pub fn create_growing_node<'a>(
    item: &'a Item<'a>,
    coverings: &'a [Covering<'a>],
) -> GrowingNode<'a> {
    let parents = item.find_parents(coverings);
    let larger = create_growing_nodes(parents, coverings);
    GrowingNode { item, larger }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::{Local, Utc};
    use surrealdb::sql::Datetime;

    use crate::{
        base_data::{ItemType, ItemVecExtensions},
        surrealdb_layer::{
            surreal_covering::SurrealCovering,
            surreal_covering_until_date_time::SurrealCoveringUntilDatetime,
            surreal_item::SurrealItem,
            surreal_required_circumstance::{CircumstanceType, SurrealRequiredCircumstance},
            SurrealTables,
        },
    };

    use super::*;

    #[test]
    fn new_to_dos_are_shown_in_next_steps() {
        let surreal_tables = SurrealTables {
            surreal_items: vec![SurrealItem {
                id: Some(("surreal_item", "1").into()),
                summary: "New item".into(),
                finished: None,
                item_type: ItemType::ToDo,
            }],
            surreal_specific_to_hopes: vec![],
            surreal_coverings: vec![],
            surreal_required_circumstances: vec![],
            surreal_coverings_until_date_time: vec![],
            surreal_specific_to_to_dos: vec![],
        };

        let items = surreal_tables.make_items();
        let coverings = surreal_tables.make_coverings(&items);
        let coverings_until_date_time = surreal_tables.make_coverings_until_date_time(&items);

        let to_dos = items.filter_just_to_dos();
        let wednesday_ignore =
            DateTime::parse_from_str("1983 Apr 13 12:09:14.274 +0000", "%Y %b %d %H:%M:%S%.3f %z")
                .unwrap()
                .into();
        let next_step_nodes = create_to_do_nodes(
            &to_dos,
            &coverings,
            &coverings_until_date_time,
            &wednesday_ignore,
            false,
        );

        assert_eq!(next_step_nodes.len(), 1);
    }

    #[test]
    fn finished_to_dos_are_not_shown() {
        let surreal_tables = SurrealTables {
            surreal_items: vec![SurrealItem {
                id: Some(("surreal_item", "1").into()),
                summary: "Finished item".into(),
                finished: Some(undefined_datetime_for_testing()),
                item_type: ItemType::ToDo,
            }],
            surreal_specific_to_hopes: vec![],
            surreal_coverings: vec![],
            surreal_required_circumstances: vec![],
            surreal_coverings_until_date_time: vec![],
            surreal_specific_to_to_dos: vec![],
        };

        let items = surreal_tables.make_items();
        let coverings = surreal_tables.make_coverings(&items);
        let coverings_until_date_time = surreal_tables.make_coverings_until_date_time(&items);

        let to_dos = items.filter_just_to_dos();
        let wednesday_ignore =
            DateTime::parse_from_str("1983 Apr 13 12:09:14.274 +0000", "%Y %b %d %H:%M:%S%.3f %z")
                .unwrap()
                .into();
        let next_step_nodes = create_to_do_nodes(
            &to_dos,
            &coverings,
            &coverings_until_date_time,
            &wednesday_ignore,
            false,
        );

        assert_eq!(next_step_nodes.len(), 0);
    }

    #[test]
    fn require_not_sunday_are_not_shown_on_sunday() {
        let surreal_items = vec![SurrealItem {
            id: Some(("surreal_item", "1").into()),
            summary: "Can't do this on Sunday".into(),
            finished: None,
            item_type: ItemType::ToDo,
        }];
        let surreal_required_circumstances = vec![SurrealRequiredCircumstance {
            id: Some(("surreal_requirement", "1").into()),
            required_for: surreal_items.first().unwrap().id.as_ref().unwrap().clone(),
            circumstance_type: CircumstanceType::NotSunday,
        }];
        let surreal_tables = SurrealTables {
            surreal_items,
            surreal_specific_to_hopes: vec![],
            surreal_coverings: vec![],
            surreal_required_circumstances,
            surreal_coverings_until_date_time: vec![],
            surreal_specific_to_to_dos: vec![],
        };

        let items = surreal_tables.make_items();
        let coverings = surreal_tables.make_coverings(&items);
        let coverings_until_date_time = surreal_tables.make_coverings_until_date_time(&items);
        let to_dos = items.filter_just_to_dos();
        let sunday =
            DateTime::parse_from_str("1983 Apr 17 12:09:14.274 +0000", "%Y %b %d %H:%M:%S%.3f %z")
                .unwrap()
                .into();
        let next_step_nodes = create_to_do_nodes(
            &to_dos,
            &coverings,
            &coverings_until_date_time,
            &sunday,
            false,
        );

        assert!(next_step_nodes.is_empty()); //Not shown because it is Sunday
    }

    #[test]
    fn require_not_sunday_are_shown_on_a_weekday() {
        let surreal_items = vec![SurrealItem {
            id: Some(("surreal_item", "1").into()),
            summary: "Can't do this on Sunday".into(),
            finished: None,
            item_type: ItemType::ToDo,
        }];
        let surreal_required_circumstances = vec![SurrealRequiredCircumstance {
            id: Some(("surreal_requirement", "1").into()),
            required_for: surreal_items.first().unwrap().id.as_ref().unwrap().clone(),
            circumstance_type: CircumstanceType::NotSunday,
        }];
        let surreal_tables = SurrealTables {
            surreal_items,
            surreal_specific_to_hopes: vec![],
            surreal_coverings: vec![],
            surreal_required_circumstances,
            surreal_coverings_until_date_time: vec![],
            surreal_specific_to_to_dos: vec![],
        };
        let items = surreal_tables.make_items();
        let coverings = surreal_tables.make_coverings(&items);
        let coverings_until_date_time = surreal_tables.make_coverings_until_date_time(&items);
        let to_dos = items.filter_just_to_dos();
        let wednesday =
            DateTime::parse_from_str("1983 Apr 13 12:09:14.274 +0000", "%Y %b %d %H:%M:%S%.3f %z")
                .unwrap()
                .into();
        let next_step_nodes = create_to_do_nodes(
            &to_dos,
            &coverings,
            &coverings_until_date_time,
            &wednesday,
            false,
        );

        assert_eq!(1, next_step_nodes.len());
    }

    #[test]
    fn require_not_sunday_are_shown_on_a_saturday() {
        let surreal_items = vec![SurrealItem {
            id: Some(("surreal_item", "1").into()),
            summary: "Can't do this on Sunday".into(),
            finished: None,
            item_type: ItemType::ToDo,
        }];
        let surreal_required_circumstances = vec![SurrealRequiredCircumstance {
            id: Some(("surreal_requirement", "1").into()),
            required_for: surreal_items.first().unwrap().id.as_ref().unwrap().clone(),
            circumstance_type: CircumstanceType::NotSunday,
        }];
        let surreal_tables = SurrealTables {
            surreal_items,
            surreal_specific_to_hopes: vec![],
            surreal_coverings: vec![],
            surreal_required_circumstances,
            surreal_coverings_until_date_time: vec![],
            surreal_specific_to_to_dos: vec![],
        };
        let items = surreal_tables.make_items();
        let to_dos = items.filter_just_to_dos();
        let coverings = surreal_tables.make_coverings(&items);
        let coverings_until_date_time = surreal_tables.make_coverings_until_date_time(&items);
        let saturday =
            DateTime::parse_from_str("1983 Apr 16 12:09:14.274 +0000", "%Y %b %d %H:%M:%S%.3f %z")
                .unwrap()
                .into();
        let next_step_nodes = create_to_do_nodes(
            &to_dos,
            &coverings,
            &coverings_until_date_time,
            &saturday,
            false,
        );

        assert_eq!(1, next_step_nodes.len());
    }

    fn undefined_datetime_for_testing() -> Datetime {
        Local::now().naive_utc().and_utc().into()
    }

    #[test]
    fn to_dos_disappear_after_they_are_covered() {
        let surreal_items = vec![
            SurrealItem {
                id: Some(("surreal_item", "1").into()),
                summary: "Covered Item that should not be shown".into(),
                finished: None,
                item_type: ItemType::ToDo,
            },
            SurrealItem {
                id: Some(("surreal_item", "2").into()),
                summary: "Covering Item that should be shown".into(),
                finished: None,
                item_type: ItemType::ToDo,
            },
        ];
        let surreal_coverings = vec![SurrealCovering {
            id: Some(("surreal_covering", "1").into()),
            smaller: surreal_items[1].id.as_ref().expect("value set").clone(),
            parent: surreal_items[0].id.as_ref().expect("value set").clone(),
        }];
        let surreal_tables = SurrealTables {
            surreal_items,
            surreal_specific_to_hopes: vec![],
            surreal_coverings,
            surreal_required_circumstances: vec![],
            surreal_coverings_until_date_time: vec![],
            surreal_specific_to_to_dos: vec![],
        };
        let items = surreal_tables.make_items();
        let coverings = surreal_tables.make_coverings(&items);
        let coverings_until_date_time = surreal_tables.make_coverings_until_date_time(&items);

        let to_dos = items.filter_just_to_dos();
        let wednesday_ignore =
            DateTime::parse_from_str("1983 Apr 13 12:09:14.274 +0000", "%Y %b %d %H:%M:%S%.3f %z")
                .unwrap()
                .into();
        let next_step_nodes = create_to_do_nodes(
            &to_dos,
            &coverings,
            &coverings_until_date_time,
            &wednesday_ignore,
            false,
        );

        assert_eq!(next_step_nodes.len(), 1);
        assert_eq!(next_step_nodes.iter().next().unwrap().to_do, &items[1]);
    }

    #[test]
    fn to_dos_return_to_next_steps_after_they_are_uncovered() {
        let surreal_items = vec![
            SurrealItem {
                id: Some(("surreal_item", "1").into()),
                summary: "Covered Item to show once the covering item is finished".into(),
                finished: None,
                item_type: ItemType::ToDo,
            },
            SurrealItem {
                id: Some(("surreal_item", "2").into()),
                summary: "Covering Item that is finished".into(),
                finished: Some(undefined_datetime_for_testing()),
                item_type: ItemType::ToDo,
            },
        ];
        let surreal_coverings = vec![SurrealCovering {
            id: Some(("surreal_covering", "1").into()),
            smaller: surreal_items[1].id.as_ref().expect("Set above").clone(),
            parent: surreal_items[0].id.as_ref().expect("Set above").clone(),
        }];
        let surreal_tables = SurrealTables {
            surreal_items,
            surreal_specific_to_hopes: vec![],
            surreal_coverings,
            surreal_required_circumstances: vec![],
            surreal_coverings_until_date_time: vec![],
            surreal_specific_to_to_dos: vec![],
        };
        let items = surreal_tables.make_items();
        let coverings = surreal_tables.make_coverings(&items);
        let coverings_until_date_time = surreal_tables.make_coverings_until_date_time(&items);

        let to_dos = items.filter_just_to_dos();
        let wednesday_ignore =
            DateTime::parse_from_str("1983 Apr 13 12:09:14.274 +0000", "%Y %b %d %H:%M:%S%.3f %z")
                .unwrap()
                .into();
        let next_step_nodes = create_to_do_nodes(
            &to_dos,
            &coverings,
            &coverings_until_date_time,
            &wednesday_ignore,
            false,
        );

        assert_eq!(next_step_nodes.len(), 1);
        assert_eq!(next_step_nodes.iter().next().unwrap().to_do, &items[0]);
    }

    #[test]
    fn to_dos_covered_until_an_exact_date_time_are_not_shown() {
        let surreal_items = vec![SurrealItem {
            id: Some(("surreal_item", "1").into()),
            summary: "Covered item that should not be shown".into(),
            finished: None,
            item_type: ItemType::ToDo,
        }];
        let now = Local::now();
        let now_utc: DateTime<Utc> = now.into();
        let in_the_future = now_utc + Duration::from_secs(60);
        let surreal_coverings_until_date_time = vec![SurrealCoveringUntilDatetime {
            id: Some(("surreal_coverings_until_date_time", "1").into()),
            cover_this: surreal_items[0].id.as_ref().unwrap().clone(),
            until: in_the_future.into(),
        }];
        let surreal_tables = SurrealTables {
            surreal_items,
            surreal_specific_to_hopes: vec![],
            surreal_coverings: vec![],
            surreal_required_circumstances: vec![],
            surreal_coverings_until_date_time,
            surreal_specific_to_to_dos: vec![],
        };

        let items: Vec<Item> = surreal_tables.make_items();
        let coverings = surreal_tables.make_coverings(&items);
        let coverings_until_date_time = surreal_tables.make_coverings_until_date_time(&items);

        let to_dos = items.filter_just_to_dos();
        let next_steps_nodes =
            create_to_do_nodes(&to_dos, &coverings, &coverings_until_date_time, &now, false);

        assert!(next_steps_nodes.is_empty());
    }

    #[test]
    fn to_dos_covered_until_an_exact_date_time_are_shown_again_after_that_time_time() {
        let surreal_items = vec![SurrealItem {
            id: Some(("surreal_item", "1").into()),
            summary: "Covered item that should not be shown".into(),
            finished: None,
            item_type: ItemType::ToDo,
        }];

        let now = Local::now();
        let now_utc: DateTime<Utc> = now.into();
        let in_the_past = now_utc - Duration::from_secs(60);

        let surreal_coverings_until_date_time = vec![SurrealCoveringUntilDatetime {
            id: Some(("surreal_coverings_until_date_time", "1").into()),
            cover_this: surreal_items[0].id.as_ref().unwrap().clone(),
            until: in_the_past.into(),
        }];
        let surreal_tables = SurrealTables {
            surreal_items,
            surreal_specific_to_hopes: vec![],
            surreal_coverings: vec![],
            surreal_required_circumstances: vec![],
            surreal_coverings_until_date_time,
            surreal_specific_to_to_dos: vec![],
        };
        let items: Vec<Item> = surreal_tables.make_items();
        let coverings = surreal_tables.make_coverings(&items);
        let coverings_until_date_time = surreal_tables.make_coverings_until_date_time(&items);

        let to_dos = items.filter_just_to_dos();
        let next_steps_nodes =
            create_to_do_nodes(&to_dos, &coverings, &coverings_until_date_time, &now, false);

        assert_eq!(1, next_steps_nodes.len());
    }
}
