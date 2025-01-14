use std::{fmt::Display, time::Duration};

use chrono::Utc;
use duration_str::parse;
use inquire::{InquireError, Select, Text};

use crate::{
    display::{
        display_duration::DisplayDuration, display_enter_list_reason::DisplayEnterListReason,
    },
    menu::YesOrNo,
    surrealdb_layer::surreal_item::{EnterListReason, Staging},
};

#[derive(Debug, Clone)]
enum EnterListReasonSelection {
    Immediately,
    DateTime,
    HighestUncovered,
}

impl Display for EnterListReasonSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnterListReasonSelection::Immediately => write!(f, "Immediately"),
            EnterListReasonSelection::DateTime => write!(f, "Wait an amount of time"),
            EnterListReasonSelection::HighestUncovered => {
                write!(f, "Once this is highest priority uncovered")
            }
        }
    }
}

impl EnterListReasonSelection {
    fn make_list_on_deck() -> Vec<Self> {
        vec![
            EnterListReasonSelection::DateTime,
            EnterListReasonSelection::Immediately,
            EnterListReasonSelection::HighestUncovered,
        ]
    }

    fn make_list_mentally_resident() -> Vec<Self> {
        vec![
            EnterListReasonSelection::DateTime,
            EnterListReasonSelection::Immediately,
        ]
    }
}

pub(crate) async fn on_deck_query() -> Result<Staging, InquireError> {
    let (enter_list, lap) = prompt_for_two_times(EnterListReasonSelection::make_list_on_deck())?;

    Ok(Staging::OnDeck {
        enter_list,
        lap: lap.into(),
    })
}

pub(crate) async fn mentally_resident_query() -> Result<Staging, InquireError> {
    let (enter_list, lap) =
        prompt_for_two_times(EnterListReasonSelection::make_list_mentally_resident())?;

    Ok(Staging::MentallyResident {
        enter_list,
        lap: lap.into(),
    })
}

fn prompt_for_two_times(
    list: Vec<EnterListReasonSelection>,
) -> Result<(EnterListReason, Duration), InquireError> {
    let now = Utc::now();
    loop {
        let selection =
            Select::new("When should this item enter the list?", list.clone()).prompt()?;
        let enter_list_reason = match selection {
            EnterListReasonSelection::Immediately => EnterListReason::DateTime(now.into()),
            EnterListReasonSelection::DateTime => {
                let return_to_string =
                    Text::new("Wait how long before returning the item to the list?").prompt()?;
                match parse(&return_to_string) {
                    Ok(return_to_duration) => {
                        let return_to = now + return_to_duration;
                        EnterListReason::DateTime(return_to.into())
                    }
                    Err(_) => match dateparser::parse(&return_to_string) {
                        Ok(return_to) => EnterListReason::DateTime(return_to.into()),
                        Err(_) => {
                            println!("Invalid input. Please try again.");
                            continue;
                        }
                    },
                }
            }
            EnterListReasonSelection::HighestUncovered => {
                let review_after =
                    Text::new("What is the maximum amount of time to wait?").prompt()?;
                match parse(&review_after) {
                    Ok(review_after_duration) => {
                        let review_after = now + review_after_duration;
                        EnterListReason::HighestUncovered {
                            earliest: now.into(),
                            review_after: review_after.into(),
                        }
                    }
                    Err(_) => match dateparser::parse(&review_after) {
                        Ok(review_after) => EnterListReason::HighestUncovered {
                            earliest: now.into(),
                            review_after: review_after.into(),
                        },
                        Err(_) => {
                            println!("Invalid input. Please try again.");
                            continue;
                        }
                    },
                }
            }
        };
        let deadline_string = Text::new("Lap length?").prompt()?;
        let lap = match parse(&deadline_string) {
            Ok(lap) => lap,
            Err(_) => {
                println!("Invalid input. Please try again.");
                continue;
            }
        };
        let result = Select::new(
            &format!(
                "Wait until: {}\n Lap: {}",
                DisplayEnterListReason::new(&enter_list_reason),
                DisplayDuration::new(&lap)
            ),
            YesOrNo::make_list(),
        )
        .prompt()?;
        match result {
            YesOrNo::Yes => return Ok((enter_list_reason, lap)),
            YesOrNo::No => continue,
        }
    }
}
