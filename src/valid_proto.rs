use std::collections::HashMap;

use protobuf::descriptor::{FieldDescriptorProto, FileDescriptorSet};
use serde_json::Value;

#[derive(Debug, PartialEq)] // PartialEq for unit testing
pub struct ValidationError {
    pub field: String, // Full path, e.g., "parent.child.field"
    pub error_type: ErrorType,
}

#[derive(Debug, PartialEq)] // PartialEq for unit testing
pub enum ErrorType {
    AdditionalField,       // Field present in JSON but not in Protobuf
    MissingField,          // Required field missing in JSON
    WrongDataType,         // Field type mismatch
    MissingArrayField,     // Empty array for a repeated field that should have data
    InvalidArrayElement,   // Array element doesn’t match expected type
    NestedValidationError, // Error in a nested message
}

/// Validates JSON against a Protobuf schema, including nested and repeated fields.
pub fn validate_json(
    file_descriptor_set: &FileDescriptorSet,
    table_name: &str,
    json_value: &Value,
    ignore_list: Vec<String>,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Build a map of message types for quick lookup by name
    let mut message_types = HashMap::new();
    for file in &file_descriptor_set.file {
        for message in &file.message_type {
            if let Some(name) = message.name.clone() {
                // Store lowercase name to make lookup case-insensitive
                message_types.insert(name.to_lowercase(), message.clone());
            }
        }
    }

    // Find the target message type and start validation
    if let Some(message) = message_types.get(&table_name.to_lowercase()) {
        // Validate the top-level message, starting with an empty path
        validate_message(
            message,
            json_value,
            &message_types,
            &ignore_list,
            "".to_string(),
            &mut errors,
        );
    } else {
        // If the table_name doesn’t match any message, report an error
        errors.push(ValidationError {
            field: table_name.to_string(),
            error_type: ErrorType::MissingField,
        });
    }

    errors
}

/// Recursively validates a message against a JSON value.
fn validate_message(
    message: &protobuf::descriptor::DescriptorProto,
    json_value: &Value,
    message_types: &HashMap<String, protobuf::descriptor::DescriptorProto>,
    ignore_list: &[String],
    parent_path: String, // Tracks the current field path (e.g., "parent.child")
    errors: &mut Vec<ValidationError>,
) {
    if let Value::Object(json_obj) = json_value {
        // Map Protobuf fields for this message by their JSON names
        let mut proto_fields = HashMap::new();
        for field in &message.field {
            if let Some(name) = field.json_name.clone() {
                if !ignore_list.contains(&name) {
                    proto_fields.insert(name, field.clone());
                }
            }
        }

        // Check JSON fields against Protobuf schema
        for (key, value) in json_obj {
            if ignore_list.contains(key) {
                continue; // Skip ignored fields
            }
            // Construct the full path for error reporting
            let field_path = if parent_path.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", parent_path, key)
            };

            if let Some(field) = proto_fields.get(key) {
                // Field exists in schema; validate its value
                validate_field(
                    field,
                    value,
                    message_types,
                    ignore_list,
                    &field_path,
                    errors,
                );
            } else {
                // Field isn’t in schema; report as additional
                errors.push(ValidationError {
                    field: field_path,
                    error_type: ErrorType::AdditionalField,
                });
            }
        }

        // Check for missing required fields in JSON
        for (name, field) in &proto_fields {
            let field_path = if parent_path.is_empty() {
                name.clone()
            } else {
                format!("{}.{}", parent_path, name)
            };
            if !json_obj.contains_key(name)
                && field.label()
                    != protobuf::descriptor::field_descriptor_proto::Label::LABEL_REPEATED
            {
                // Report missing non-repeated fields (repeated fields can be empty)
                errors.push(ValidationError {
                    field: field_path,
                    error_type: ErrorType::MissingField,
                });
            }
        }
    } else {
        // JSON should be an object for a message; report type mismatch
        errors.push(ValidationError {
            field: parent_path,
            error_type: ErrorType::WrongDataType,
        });
    }
}

/// Validates a single Protobuf field against its JSON value.
fn validate_field(
    field: &FieldDescriptorProto,
    value: &Value,
    message_types: &HashMap<String, protobuf::descriptor::DescriptorProto>,
    ignore_list: &[String],
    field_path: &str,
    errors: &mut Vec<ValidationError>,
) {
    match field.label() {
        protobuf::descriptor::field_descriptor_proto::Label::LABEL_REPEATED => {
            // Handle repeated fields, which map to JSON arrays
            if let Value::Array(arr) = value {
                if arr.is_empty()
                    && field.type_()
                        == protobuf::descriptor::field_descriptor_proto::Type::TYPE_MESSAGE
                {
                    // Warn if a repeated message field is empty (optional rule)
                    errors.push(ValidationError {
                        field: field_path.to_string(),
                        error_type: ErrorType::MissingArrayField,
                    });
                }
                // Validate each array element
                for (i, item) in arr.iter().enumerate() {
                    let item_path = format!("{}[{}]", field_path, i);
                    if field.type_()
                        == protobuf::descriptor::field_descriptor_proto::Type::TYPE_MESSAGE
                    {
                        // Nested message in a repeated field
                        if let Some(type_name) = field.type_name.clone() {
                            let clean_type_name = type_name.trim_start_matches('.').to_lowercase();
                            if let Some(nested_message) = message_types.get(&clean_type_name) {
                                // Recursively validate the nested message
                                validate_message(
                                    nested_message,
                                    item,
                                    message_types,
                                    ignore_list,
                                    item_path,
                                    errors,
                                );
                            }
                        }
                    } else {
                        // Primitive type in repeated field
                        if !is_valid_primitive(field.type_(), item) {
                            errors.push(ValidationError {
                                field: item_path,
                                error_type: ErrorType::InvalidArrayElement,
                            });
                        }
                    }
                }
            } else {
                // Repeated field should be an array; report type mismatch
                errors.push(ValidationError {
                    field: field_path.to_string(),
                    error_type: ErrorType::WrongDataType,
                });
            }
        }
        _ => {
            // Handle non-repeated fields
            if field.type_() == protobuf::descriptor::field_descriptor_proto::Type::TYPE_MESSAGE {
                // Nested message field
                if let Some(type_name) = field.type_name.clone() {
                    let clean_type_name = type_name.trim_start_matches('.').to_lowercase();
                    if let Some(nested_message) = message_types.get(&clean_type_name) {
                        // Recursively validate the nested message
                        validate_message(
                            nested_message,
                            value,
                            message_types,
                            ignore_list,
                            field_path.to_string(),
                            errors,
                        );
                    }
                }
            } else {
                // Primitive type field
                if !is_valid_primitive(field.type_(), value) {
                    errors.push(ValidationError {
                        field: field_path.to_string(),
                        error_type: ErrorType::WrongDataType,
                    });
                }
            }
        }
    }
}

/// Checks if a JSON value matches a Protobuf primitive type.
fn is_valid_primitive(
    field_type: protobuf::descriptor::field_descriptor_proto::Type,
    value: &Value,
) -> bool {
    match (field_type, value) {
        // String field should be a JSON string
        (protobuf::descriptor::field_descriptor_proto::Type::TYPE_STRING, Value::String(_)) => true,
        // Int32 field should be a JSON number that fits in i64
        (protobuf::descriptor::field_descriptor_proto::Type::TYPE_INT32, Value::Number(n)) => {
            n.is_i64()
        }
        // Float field can be any JSON number
        (protobuf::descriptor::field_descriptor_proto::Type::TYPE_FLOAT, Value::Number(_)) => true,
        // Bool field should be a JSON boolean
        (protobuf::descriptor::field_descriptor_proto::Type::TYPE_BOOL, Value::Bool(_)) => true,
        _ => false, // Any other combination is invalid
    }
}

// Unit tests
#[cfg(test)]
mod tests {
    use super::*;
    use protobuf::{
        descriptor::{DescriptorProto, FieldDescriptorProto},
        EnumOrUnknown,
    };
    use serde_json::json;

    fn create_test_descriptor() -> FileDescriptorSet {
        // Create a mock FileDescriptorSet with nested and repeated fields, including SubDescriptorProto
        let mut file_set = FileDescriptorSet::new();
        let mut file = protobuf::descriptor::FileDescriptorProto::new();
        file.name = Some("FileLevel.proto".to_string());

        // Define TopLevel message
        let mut top_level = DescriptorProto::new();
        top_level.name = Some("TopLevel".to_string());
        let mut name_field = FieldDescriptorProto::new();
        name_field.name = Some("name".to_string());
        name_field.json_name = Some("name".to_string());
        name_field.type_ = Some(EnumOrUnknown::new(
            protobuf::descriptor::field_descriptor_proto::Type::TYPE_STRING,
        ));
        let mut items_field = FieldDescriptorProto::new();
        items_field.name = Some("items".to_string());
        items_field.json_name = Some("items".to_string());
        items_field.label = Some(EnumOrUnknown::new(
            protobuf::descriptor::field_descriptor_proto::Label::LABEL_REPEATED,
        ));
        items_field.type_name = Some(".SubMessage".to_string());
        items_field.type_ = Some(EnumOrUnknown::new(
            protobuf::descriptor::field_descriptor_proto::Type::TYPE_MESSAGE,
        ));
        top_level.field.push(name_field);
        top_level.field.push(items_field);

        // Define SubMessage (contains a repeated SubDescriptorProto)
        let mut sub_message = DescriptorProto::new();
        sub_message.name = Some("SubMessage".to_string());
        let mut id_field = FieldDescriptorProto::new();
        id_field.name = Some("id".to_string());
        id_field.json_name = Some("id".to_string());
        id_field.type_ = Some(EnumOrUnknown::new(
            protobuf::descriptor::field_descriptor_proto::Type::TYPE_INT32,
        ));
        let mut desc_field = FieldDescriptorProto::new();
        desc_field.name = Some("description".to_string());
        desc_field.json_name = Some("description".to_string());
        desc_field.type_ = Some(EnumOrUnknown::new(
            protobuf::descriptor::field_descriptor_proto::Type::TYPE_STRING,
        ));
        let mut details_field = FieldDescriptorProto::new();
        details_field.name = Some("details".to_string());
        details_field.json_name = Some("details".to_string());
        details_field.label = Some(EnumOrUnknown::new(
            protobuf::descriptor::field_descriptor_proto::Label::LABEL_REPEATED,
        ));
        details_field.type_name = Some(".SubDescriptorProto".to_string());
        details_field.type_ = Some(EnumOrUnknown::new(
            protobuf::descriptor::field_descriptor_proto::Type::TYPE_MESSAGE,
        ));
        sub_message.field.push(id_field);
        sub_message.field.push(desc_field);
        sub_message.field.push(details_field);

        // Define SubDescriptorProto (new nested message)
        let mut sub_descriptor = DescriptorProto::new();
        sub_descriptor.name = Some("SubDescriptorProto".to_string());
        let mut value_field = FieldDescriptorProto::new();
        value_field.name = Some("value".to_string());
        value_field.json_name = Some("value".to_string());
        value_field.type_ = Some(EnumOrUnknown::new(
            protobuf::descriptor::field_descriptor_proto::Type::TYPE_STRING,
        ));
        sub_descriptor.field.push(value_field);

        file.message_type.push(top_level);
        file.message_type.push(sub_message);
        file.message_type.push(sub_descriptor);
        file_set.file.push(file);

        file_set
    }

    #[test]
    fn test_nested_and_repeated_validation_with_sub_descriptor() {
        let file_set = create_test_descriptor();

        // Test JSON with nested and repeated fields, including the new SubDescriptorProto
        let json_value = json!({
            "name": "test",
            "items": [
                {
                    "id": 1,
                    "description": "first",
                    "details": [
                        {"value": "a"},
                        {"value": 42} // Invalid type (should be string)
                    ]
                },
                {
                    "id": "two", // Invalid type (should be int32)
                    "description": "second",
                    "details": [
                        {"value": "b"},
                        {"extra": "field"} // Additional field
                    ]
                }
            ],
            "extra": "field" // Additional field
        });

        let errors = validate_json(&file_set, "TopLevel", &json_value, vec![]);

        // Expected errors
        assert_eq!(errors.len(), 5); // Five distinct errors
        assert_eq!(
            errors[0],
            ValidationError {
                field: "extra".to_string(),
                error_type: ErrorType::AdditionalField,
            }
        );
        assert_eq!(
            errors[1],
            ValidationError {
                field: "items[0].details[1].value".to_string(),
                error_type: ErrorType::WrongDataType,
            }
        );
        assert_eq!(
            errors[2],
            ValidationError {
                field: "items[1].id".to_string(),
                error_type: ErrorType::InvalidArrayElement,
            }
        );
        assert_eq!(
            errors[3],
            ValidationError {
                field: "items[1].details[1].extra".to_string(),
                error_type: ErrorType::AdditionalField,
            }
        );
    }
}
