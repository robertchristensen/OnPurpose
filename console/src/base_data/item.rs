use chrono::{DateTime, Datelike, Local};
use surrealdb::{
    opt::RecordId,
    sql::{Datetime, Thing},
};

use crate::surrealdb_layer::{
    surreal_item::SurrealItem,
    surreal_required_circumstance::{CircumstanceType, SurrealRequiredCircumstance},
    surreal_specific_to_hope::{SurrealSpecificToHope, SurrealSpecificToHopes},
};

use super::{
    hope::Hope, motivation::Motivation, to_do::ToDo, Covering, CoveringUntilDateTime, ItemType,
};

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Item<'a> {
    pub id: &'a Thing,
    pub summary: &'a String,
    pub finished: &'a Option<Datetime>,
    pub item_type: &'a ItemType,
    pub required_circumstances: Vec<&'a SurrealRequiredCircumstance>,
    pub surreal_item: &'a SurrealItem,
}

impl<'a> From<&'a Item<'a>> for &'a SurrealItem {
    fn from(value: &Item<'a>) -> Self {
        value.surreal_item
    }
}

impl From<Item<'_>> for SurrealItem {
    fn from(value: Item<'_>) -> Self {
        value.surreal_item.clone()
    }
}

pub trait ItemVecExtensions {
    fn lookup_from_record_id<'a>(&'a self, record_id: &RecordId) -> Option<&'a Item>;
    fn filter_just_to_dos(&self) -> Vec<ToDo<'_>>;
    fn filter_just_hopes<'a>(
        &'a self,
        surreal_specific_to_hopes: &'a [SurrealSpecificToHope],
    ) -> Vec<Hope<'a>>;
    fn filter_just_motivations(&self) -> Vec<Motivation<'_>>;
}

impl<'b> ItemVecExtensions for [Item<'b>] {
    fn lookup_from_record_id<'a>(&'a self, record_id: &RecordId) -> Option<&'a Item> {
        self.iter().find(|x| x.id == record_id)
    }

    fn filter_just_to_dos(&self) -> Vec<ToDo<'_>> {
        self.iter()
            .filter_map(|x| {
                if x.item_type == &ItemType::ToDo {
                    Some(ToDo::new(x))
                } else {
                    None
                }
            })
            .collect()
    }

    fn filter_just_hopes<'a>(
        &'a self,
        surreal_specific_to_hopes: &'a [SurrealSpecificToHope],
    ) -> Vec<Hope<'a>> {
        //Initially I had this with a iter().filter_map() but then I had some issue with the borrow checker and surreal_specific_to_hopes that I didn't understand so I refactored it to this code to work around that issue
        let mut just_hopes = Vec::default();
        for x in self.iter() {
            if x.item_type == &ItemType::Hope {
                let hope_specific: Option<&SurrealSpecificToHope> =
                    surreal_specific_to_hopes.get_by_id(x.id);
                let hope_specific = hope_specific.unwrap().clone(); //TODO: Figure out how to use borrow rather than clone()
                let hope = Hope::new(x, hope_specific);
                just_hopes.push(hope);
            }
        }
        just_hopes
    }

    fn filter_just_motivations(&self) -> Vec<Motivation<'_>> {
        self.iter()
            .filter_map(|x| {
                if x.item_type == &ItemType::Motivation {
                    Some(Motivation::new(x))
                } else {
                    None
                }
            })
            .collect()
    }
}

impl<'b> Item<'b> {
    pub(crate) fn new(
        surreal_item: &'b SurrealItem,
        required_circumstances: Vec<&'b SurrealRequiredCircumstance>,
    ) -> Self {
        Self {
            id: surreal_item.id.as_ref().expect("Already in DB"),
            summary: &surreal_item.summary,
            finished: &surreal_item.finished,
            item_type: &surreal_item.item_type,
            required_circumstances,
            surreal_item,
        }
    }

    pub fn is_circumstances_met(&self, date: &DateTime<Local>, are_we_in_focus_time: bool) -> bool {
        self.is_circumstances_met_sunday(date)
            && self.is_circumstances_met_focus_time(are_we_in_focus_time)
    }

    pub fn is_circumstances_met_sunday(&self, date: &DateTime<Local>) -> bool {
        !self
            .required_circumstances
            .iter()
            .any(|x| match x.circumstance_type {
                CircumstanceType::NotSunday => date.weekday().num_days_from_sunday() == 0,
                _ => false,
            })
    }

    pub fn is_circumstances_met_focus_time(&self, are_we_in_focus_time: bool) -> bool {
        let should_this_be_done_during_focus_time = self
            .required_circumstances
            .iter()
            .any(|x| matches!(x.circumstance_type, CircumstanceType::DuringFocusTime));

        should_this_be_done_during_focus_time == are_we_in_focus_time
    }

    pub fn is_finished(&self) -> bool {
        self.finished.is_some()
    }

    pub fn is_covered_by_another_item(&self, coverings: &[Covering<'_>]) -> bool {
        let mut covered_by = coverings.iter().filter(|x| self == x.parent);
        //Now see if the items that are covering are finished or active
        covered_by.any(|x| !x.smaller.is_finished())
    }

    pub fn is_covered_by_date_time(
        &self,
        coverings_until_date_time: &[CoveringUntilDateTime<'_>],
        now: &DateTime<Local>,
    ) -> bool {
        let mut covered_by_date_time = coverings_until_date_time
            .iter()
            .filter(|x| self == x.cover_this);
        covered_by_date_time.any(|x| now < &x.until)
    }

    pub fn is_covered(
        &self,
        coverings: &[Covering<'_>],
        coverings_until_date_time: &[CoveringUntilDateTime<'_>],
        now: &DateTime<Local>,
    ) -> bool {
        self.is_covered_by_another_item(coverings)
            || self.is_covered_by_date_time(coverings_until_date_time, now)
    }

    pub fn covered_by<'a>(&self, coverings: &[Covering<'a>]) -> Vec<&'a Item<'a>> {
        coverings
            .iter()
            .filter_map(|x| {
                if x.parent == self && !x.smaller.is_finished() {
                    Some(x.smaller)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn who_am_i_covering<'a>(&self, coverings: &[Covering<'a>]) -> Vec<&'a Item<'a>> {
        coverings
            .iter()
            .filter_map(|x| {
                if x.smaller == self {
                    Some(x.parent)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn get_surreal_item(&self) -> &'b SurrealItem {
        self.surreal_item
    }

    pub fn get_summary(&self) -> &'b str {
        self.summary
    }
}

#[cfg(test)]
mod tests {
    use crate::surrealdb_layer::surreal_required_circumstance::CircumstanceType;

    use super::*;

    #[test]
    fn is_circumstances_met_returns_false_if_circumstance_type_is_not_sunday_and_it_is_sunday() {
        let surreal_item = SurrealItem {
            id: Some(("surreal_item", "1").into()),
            summary: "Circumstance type is not Sunday".into(),
            finished: None,
            item_type: ItemType::ToDo,
            smaller_items_in_priority_order: Vec::default(),
        };

        let required_circumstance = SurrealRequiredCircumstance {
            id: Some(("surreal_required_circumstance", "1").into()),
            required_for: surreal_item.id.as_ref().expect("set above").clone(),
            circumstance_type: CircumstanceType::NotSunday,
        };

        let item = Item::new(&surreal_item, vec![&required_circumstance]);

        let sunday =
            DateTime::parse_from_str("1983 Apr 17 12:09:14.274 +0000", "%Y %b %d %H:%M:%S%.3f %z")
                .unwrap()
                .into();

        assert!(!item.is_circumstances_met(&sunday, false));
    }

    #[test]
    fn is_circumstances_met_returns_true_if_circumstance_type_is_not_sunday_and_it_is_not_sunday() {
        let surreal_item = SurrealItem {
            id: Some(("surreal_item", "1").into()),
            summary: "Circumstance type is not Sunday".into(),
            finished: None,
            item_type: ItemType::ToDo,
            smaller_items_in_priority_order: Vec::default(),
        };

        let required_circumstance = SurrealRequiredCircumstance {
            id: Some(("surreal_required_circumstance", "1").into()),
            required_for: surreal_item.id.as_ref().expect("set above").clone(),
            circumstance_type: CircumstanceType::NotSunday,
        };

        let item = Item::new(&surreal_item, vec![&required_circumstance]);

        let wednesday =
            DateTime::parse_from_str("1983 Apr 13 12:09:14.274 +0000", "%Y %b %d %H:%M:%S%.3f %z")
                .unwrap()
                .into();

        assert!(item.is_circumstances_met(&wednesday, false));
    }

    #[test]
    fn is_circumstances_met_returns_false_if_focus_time_is_not_active_and_circumstance_type_is_during_focus_time(
    ) {
        let surreal_item = SurrealItem {
            id: Some(("surreal_item", "1").into()),
            summary: "Circumstance type is not Sunday".into(),
            finished: None,
            item_type: ItemType::ToDo,
            smaller_items_in_priority_order: Vec::default(),
        };

        let required_circumstance = SurrealRequiredCircumstance {
            id: Some(("surreal_required_circumstance", "1").into()),
            required_for: surreal_item.id.as_ref().expect("set above").clone(),
            circumstance_type: CircumstanceType::DuringFocusTime,
        };

        let item = Item::new(&surreal_item, vec![&required_circumstance]);

        let wednesday_ignore =
            DateTime::parse_from_str("1983 Apr 13 12:09:14.274 +0000", "%Y %b %d %H:%M:%S%.3f %z")
                .unwrap()
                .into();

        assert!(item.is_circumstances_met(&wednesday_ignore, true));
    }

    #[test]
    fn is_circumstances_met_returns_true_if_focus_time_is_active_and_circumstance_type_is_during_focus_time(
    ) {
        let surreal_item = SurrealItem {
            id: Some(("surreal_item", "1").into()),
            summary: "Circumstance type is not Sunday".into(),
            finished: None,
            item_type: ItemType::ToDo,
            smaller_items_in_priority_order: Vec::default(),
        };

        let required_circumstance = SurrealRequiredCircumstance {
            id: Some(("surreal_required_circumstance", "1").into()),
            required_for: surreal_item.id.as_ref().expect("set above").clone(),
            circumstance_type: CircumstanceType::DuringFocusTime,
        };

        let item = Item::new(&surreal_item, vec![&required_circumstance]);

        let wednesday_ignore =
            DateTime::parse_from_str("1983 Apr 13 12:09:14.274 +0000", "%Y %b %d %H:%M:%S%.3f %z")
                .unwrap()
                .into();

        assert!(!item.is_circumstances_met(&wednesday_ignore, false));
    }
}
