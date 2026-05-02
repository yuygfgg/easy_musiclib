use crate::domain;
use easy_musiclib_shared as api;

impl<T, U> From<api::ListResponse<T>> for domain::ListPage<U>
where
    U: From<T>,
{
    fn from(value: api::ListResponse<T>) -> Self {
        Self {
            items: value.items.into_iter().map(Into::into).collect(),
            next_cursor: value.next_cursor.map(domain::EntityId::new),
            total: value.total,
        }
    }
}

impl<T, U> From<domain::ListPage<T>> for api::ListResponse<U>
where
    T: Into<U>,
{
    fn from(value: domain::ListPage<T>) -> Self {
        Self {
            items: value.items.into_iter().map(Into::into).collect(),
            next_cursor: value.next_cursor.map(domain::EntityId::raw),
            total: value.total,
        }
    }
}
