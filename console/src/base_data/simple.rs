use super::item::Item;

pub(crate) struct Simple<'s> {
    pub(crate) item: &'s Item<'s>,
}

impl<'s> Simple<'s> {
    pub(crate) fn new(item: &'s Item<'s>) -> Self {
        Self { item }
    }

    pub(crate) fn get_item(&'s self) -> &'s Item<'s> {
        self.item
    }
}