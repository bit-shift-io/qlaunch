use serde_json::Value;

use super::{SCHEMA_VERSION, StoreError};

/// Brings a document written under `schema` up to [`SCHEMA_VERSION`].
///
/// Version 1 is the first schema, so there is nothing older to migrate from,
/// and a newer document was written by a version of the program that knows
/// things this one does not, so it must not be read as v1. Both are refused.
/// When a v2 exists, its upgrade from v1 is added here.
pub(crate) fn migrate(schema: u32, doc: Value) -> Result<Value, StoreError> {
    match schema {
        SCHEMA_VERSION => Ok(doc),
        other => Err(StoreError::UnsupportedSchema(other)),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn the_current_schema_passes_through_unchanged() {
        let doc = json!({ "schema": SCHEMA_VERSION, "x": [1, 2] });
        assert_eq!(migrate(SCHEMA_VERSION, doc.clone()).unwrap(), doc);
    }

    #[test]
    fn older_and_newer_schemas_are_refused() {
        for schema in [0, SCHEMA_VERSION + 1, 99, u32::MAX] {
            let result = migrate(schema, json!({ "schema": schema }));
            assert!(
                matches!(result, Err(StoreError::UnsupportedSchema(n)) if n == schema),
                "schema {schema}"
            );
        }
    }
}
