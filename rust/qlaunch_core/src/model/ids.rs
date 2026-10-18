use uuid::Uuid;

pub type Id = String;

pub(crate) fn new_id() -> Id {
    Uuid::new_v4().to_string()
}
