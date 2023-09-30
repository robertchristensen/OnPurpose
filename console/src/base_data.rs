use chrono::{DateTime, Datelike, Local, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::{
    opt::RecordId,
    sql::{Datetime, Thing},
};
use surrealdb_extra::table::Table;

#[derive(PartialEq, Eq, Table, Serialize, Deserialize, Clone, Debug)]
#[table(name = "item")]
pub struct SurrealItem {
    pub id: Option<Thing>,
    pub summary: String,
    pub finished: Option<Datetime>,
    pub item_type: ItemType,
}

pub trait SurrealItemVecExtensions {
    fn make_items<'a>(&'a self, requirements: &'a [SurrealRequirement]) -> Vec<Item<'a>>;
}

impl SurrealItemVecExtensions for [SurrealItem] {
    fn make_items<'a>(&'a self, requirements: &'a [SurrealRequirement]) -> Vec<Item<'a>> {
        self.iter().map(|x| x.make_item(requirements)).collect()
    }
}

impl SurrealItem {
    pub fn make_item<'a>(&'a self, requirements: &'a [SurrealRequirement]) -> Item<'a> {
        let my_requirements = requirements
            .iter()
            .filter(|x| {
                &x.requirement_for
                    == self
                        .id
                        .as_ref()
                        .expect("Item should already be in the database and have an id")
            })
            .collect();

        Item {
            id: self
                .id
                .as_ref()
                .expect("Item should already be in the database and have an id"),
            summary: &self.summary,
            finished: &self.finished,
            item_type: &self.item_type,
            requirements: my_requirements,
            surreal_item: self,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Item<'a> {
    pub id: &'a Thing,
    pub summary: &'a String,
    pub finished: &'a Option<Datetime>,
    pub item_type: &'a ItemType,
    pub requirements: Vec<&'a SurrealRequirement>,
    surreal_item: &'a SurrealItem,
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

impl From<SurrealItem> for Option<Thing> {
    fn from(value: SurrealItem) -> Self {
        value.id
    }
}

impl<'a> From<ToDo<'a>> for SurrealItem {
    fn from(value: ToDo<'a>) -> Self {
        value.item.surreal_item.clone()
    }
}

pub trait ItemVecExtensions {
    fn lookup_from_record_id<'a>(&'a self, record_id: &RecordId) -> Option<&'a Item>;
    fn filter_just_to_dos(&self) -> Vec<ToDo<'_>>;
    fn filter_just_hopes(&self) -> Vec<Hope<'_>>;
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
                    Some(ToDo {
                        id: x.id,
                        summary: x.summary,
                        finished: x.finished,
                        item: x,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn filter_just_hopes(&self) -> Vec<Hope<'_>> {
        self.iter()
            .filter_map(|x| {
                if x.item_type == &ItemType::Hope {
                    Some(Hope {
                        id: x.id,
                        summary: x.summary,
                        finished: x.finished,
                        item: x,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn filter_just_motivations(&self) -> Vec<Motivation<'_>> {
        self.iter()
            .filter_map(|x| {
                if x.item_type == &ItemType::Motivation {
                    Some(Motivation {
                        id: x.id,
                        summary: x.summary,
                        finished: x.finished,
                        item: x,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

impl<'b> Item<'b> {
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
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Clone, Debug)]
pub enum ItemType {
    Question,
    ToDo,
    Hope,
    Motivation,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ToDo<'a> {
    pub id: &'a Thing,
    pub summary: &'a String,
    pub finished: &'a Option<Datetime>,
    item: &'a Item<'a>,
}

impl<'a> From<ToDo<'a>> for &'a Thing {
    fn from(value: ToDo<'a>) -> Self {
        value.id
    }
}

impl<'a> From<&ToDo<'a>> for &'a SurrealItem {
    fn from(value: &ToDo<'a>) -> Self {
        value.item.into()
    }
}

impl<'a> From<&ToDo<'a>> for &'a Item<'a> {
    fn from(value: &ToDo<'a>) -> Self {
        value.item
    }
}

impl<'a> From<ToDo<'a>> for Item<'a> {
    fn from(value: ToDo<'a>) -> Self {
        value.item.clone()
    }
}

impl<'a> PartialEq<Item<'a>> for ToDo<'a> {
    fn eq(&self, other: &Item<'a>) -> bool {
        self.item == other
    }
}

impl<'a> ToDo<'a> {
    pub fn is_covered_by_another_item(&self, coverings: &[Covering<'_>]) -> bool {
        self.item.is_covered_by_another_item(coverings)
    }

    pub fn is_covered_by_date_time(
        &self,
        coverings_until_date_time: &[CoveringUntilDateTime<'_>],
        now: &DateTime<Local>,
    ) -> bool {
        self.item
            .is_covered_by_date_time(coverings_until_date_time, now)
    }

    pub fn is_covered(
        &self,
        coverings: &[Covering<'_>],
        coverings_until_date_time: &[CoveringUntilDateTime<'_>],
        now: &DateTime<Local>,
    ) -> bool {
        self.item
            .is_covered(coverings, coverings_until_date_time, now)
    }

    pub fn is_finished(&self) -> bool {
        self.finished.is_some()
    }

    pub fn is_requirements_met(&self, date: &DateTime<Local>) -> bool {
        !self
            .item
            .requirements
            .iter()
            .any(|x| match x.requirement_type {
                RequirementType::NotSunday => date.weekday().num_days_from_sunday() == 0,
            })
    }

    pub fn get_surreal_item(&self) -> &'a SurrealItem {
        self.item.surreal_item
    }
}

/// Could have a review_type with options for Milestone, StoppingPoint, and ReviewPoint
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Hope<'a> {
    pub id: &'a Thing,
    pub summary: &'a String,
    pub finished: &'a Option<Datetime>,
    item: &'a Item<'a>,
}

impl<'a> From<Hope<'a>> for Thing {
    fn from(value: Hope) -> Self {
        value.id.clone()
    }
}

impl<'a> From<&'a Hope<'a>> for &'a SurrealItem {
    fn from(value: &'a Hope<'a>) -> Self {
        value.item.into()
    }
}

impl PartialEq<Hope<'_>> for Item<'_> {
    fn eq(&self, other: &Hope<'_>) -> bool {
        self == other.item
    }
}

impl PartialEq<Item<'_>> for Hope<'_> {
    fn eq(&self, other: &Item) -> bool {
        self.item == other
    }
}

impl<'a> Hope<'a> {
    pub fn is_finished(&self) -> bool {
        self.finished.is_some()
    }

    pub fn is_covered_by_another_hope(&self, coverings: &[Covering<'_>]) -> bool {
        let mut covered_by = coverings.iter().filter(|x| {
            self == x.parent && x.smaller.item_type == &ItemType::Hope && !x.smaller.is_finished()
        });
        //Now see if the items that are covering are finished or active
        covered_by.any(|x| !x.smaller.is_finished())
    }

    pub fn is_covered_by_another_item(&self, coverings: &[Covering<'_>]) -> bool {
        self.item.is_covered_by_another_item(coverings)
    }

    pub fn is_covered_by_date_time(
        &self,
        coverings_until_date_time: &[CoveringUntilDateTime<'_>],
        now: &DateTime<Local>,
    ) -> bool {
        self.item
            .is_covered_by_date_time(coverings_until_date_time, now)
    }

    pub fn is_covered(
        &self,
        coverings: &[Covering<'_>],
        coverings_until_date_time: &[CoveringUntilDateTime<'_>],
        now: &DateTime<Local>,
    ) -> bool {
        self.item
            .is_covered(coverings, coverings_until_date_time, now)
    }

    pub fn covered_by(&self, coverings: &[Covering<'a>]) -> Vec<&'a Item<'a>> {
        self.item.covered_by(coverings)
    }

    pub fn who_am_i_covering(&self, coverings: &[Covering<'a>]) -> Vec<&'a Item<'a>> {
        self.item.who_am_i_covering(coverings)
    }

    pub fn get_surreal_item(&self) -> &'a SurrealItem {
        self.item.surreal_item
    }

    pub fn get_item(&self) -> &'a Item<'a> {
        self.item
    }
}

/// Could have a motivation_type with options for Commitment (do it because the outcome of doing it is wanted), Obligation (do it because the consequence of not doing it is bad), or Value
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Motivation<'a> {
    pub id: &'a Thing,
    pub summary: &'a String,
    pub finished: &'a Option<Datetime>,
    item: &'a Item<'a>,
}

impl<'a> From<Motivation<'a>> for Thing {
    fn from(value: Motivation<'a>) -> Self {
        value.id.clone()
    }
}

impl<'a> Motivation<'a> {
    pub fn is_finished(&self) -> bool {
        self.finished.is_some()
    }
}

pub struct Covering<'a> {
    pub smaller: &'a Item<'a>,
    pub parent: &'a Item<'a>,
}

#[derive(PartialEq, Eq, Table, Serialize, Deserialize, Clone, Debug)]
#[table(name = "coverings")]
pub struct SurrealCovering {
    pub id: Option<Thing>,
    pub smaller: RecordId,
    pub parent: RecordId,
}

impl<'a> From<Covering<'a>> for SurrealCovering {
    fn from(value: Covering<'a>) -> Self {
        SurrealCovering {
            id: None,
            smaller: value.smaller.id.clone(),
            parent: value.parent.id.clone(),
        }
    }
}

pub trait SurrealCoveringVecExtensions {
    fn make_coverings<'a>(&self, items: &'a [Item<'a>]) -> Vec<Covering<'a>>;
}

impl SurrealCoveringVecExtensions for Vec<SurrealCovering> {
    fn make_coverings<'a>(&self, items: &'a [Item<'a>]) -> Vec<Covering<'a>> {
        self.iter()
            .map(|x| Covering {
                smaller: items.lookup_from_record_id(&x.smaller).unwrap(),
                parent: items.lookup_from_record_id(&x.parent).unwrap(),
            })
            .collect()
    }
}

/// The purpose of this struct is to record Items that should be covered for a certain amount of time or until
/// an exact date_time
#[derive(PartialEq, Eq, Table, Serialize, Deserialize, Clone, Debug)]
#[table(name = "coverings_until_datetime")]
pub struct SurrealCoveringUntilDatetime {
    pub id: Option<Thing>,
    pub cover_this: RecordId,
    pub until: Datetime,
}

pub trait SurrealCoveringUntilDatetimeVecExtensions {
    fn make_coverings_until_date_time<'a>(
        &'a self,
        items: &'a [Item<'a>],
    ) -> Vec<CoveringUntilDateTime<'a>>;
}

impl SurrealCoveringUntilDatetimeVecExtensions for Vec<SurrealCoveringUntilDatetime> {
    fn make_coverings_until_date_time<'a>(
        &'a self,
        items: &'a [Item<'a>],
    ) -> Vec<CoveringUntilDateTime<'a>> {
        self.iter()
            .map(|x| {
                let until_utc: DateTime<Utc> = x.until.clone().into();
                CoveringUntilDateTime {
                    cover_this: items.lookup_from_record_id(&x.cover_this).unwrap(),
                    until: until_utc.into(),
                }
            })
            .collect()
    }
}

pub struct CoveringUntilDateTime<'a> {
    pub cover_this: &'a Item<'a>,
    pub until: DateTime<Local>,
}

#[derive(PartialEq, Eq, Table, Serialize, Deserialize, Clone, Debug)]
#[table(name = "processed_text")]
pub struct ProcessedText {
    pub id: Option<Thing>,
    pub text: String,
    pub when_written: Datetime,
    pub for_item: RecordId,
}

impl Item<'_> {
    pub fn find_parents<'a>(&self, linkage: &'a [Covering<'a>]) -> Vec<&'a Item<'a>> {
        linkage
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
}

#[derive(PartialEq, Eq, Table, Serialize, Deserialize, Clone, Debug)]
#[table(name = "requirements")]
pub struct SurrealRequirement {
    pub id: Option<Thing>,
    pub requirement_for: RecordId,
    pub requirement_type: RequirementType,
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Clone, Debug)]
pub enum RequirementType {
    NotSunday,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Requirement<'a> {
    pub requirement_for: &'a SurrealItem,
    pub requirement_type: &'a RequirementType,
    surreal_requirement: &'a SurrealRequirement,
}

impl<'a> From<&Requirement<'a>> for &'a SurrealRequirement {
    fn from(value: &Requirement<'a>) -> Self {
        value.surreal_requirement
    }
}

impl<'a> From<Requirement<'a>> for &'a SurrealRequirement {
    fn from(value: Requirement<'a>) -> Self {
        value.surreal_requirement
    }
}
