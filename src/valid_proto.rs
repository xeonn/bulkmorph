use std::collections::HashSet;

use protobuf::descriptor::FileDescriptorSet;
use serde_json::Value;


#[derive(Debug)]
pub struct ValidationError {
    pub field: String,
    pub error_type: ErrorType,
}

#[derive(Debug)]
pub enum ErrorType {
    AdditionalField,
    MissingField,
    WrongDataType,
    MissingArrayField,
}

/// Validates JSON against a Protobuf schema.
pub fn validate_json(
    file_descriptor_set: &FileDescriptorSet,
    table_name: &str,
    json_value: &Value,
    ignore_list: Vec<String>,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Extract field names and types from the Protobuf schema
    let mut proto_fields = HashSet::new();
    for file in &file_descriptor_set.file {
        for message in &file.message_type {
            if message.name == None {
                continue;
            }

            let msgname = message.name.clone().unwrap().to_lowercase();
            if msgname != table_name.to_string().to_lowercase() {
                // Skip messages that don't match the table name
                continue;
            }

            for field in &message.field {
                if field.name == None {
                    continue;
                }
                if ignore_list.contains(&field.name.clone().unwrap()) {
                    continue;
                }
                proto_fields.insert(field.json_name.clone());
            }
        }
    }

    // Validate JSON fields
    if let Value::Object(json_obj) = json_value {
        for (key, _value) in json_obj {
            let k = Some(key.clone());

            if ignore_list.contains(&key) {
                continue;
            }

            // Check if the field is missing in the Protobuf schema
            if !proto_fields.contains(&k) {
                errors.push(ValidationError {
                    field: key.clone(),
                    error_type: ErrorType::AdditionalField,
                });
            }
        }
    }

    // Check if any required fields are missing in the JSON
    for field in proto_fields {
        if field == None {
            continue;
        }

        let field = field.unwrap();

        if let Value::Object(json_obj) = json_value {
            if !json_obj.contains_key(&field) {
                errors.push(ValidationError {
                    field: field.clone(),
                    error_type: ErrorType::MissingField,
                });
            }
        }
    }

    errors
}
