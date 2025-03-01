mod args;
mod fetch;
mod valid_proto;

use std::{fs, path::Path, sync::Arc};

use fetch::Fetch;
use mlua::{Function, Lua};
use protobuf::descriptor::FileDescriptorSet;
use protobuf_parse::Parser;
use reqwest::{Client, StatusCode};
use serde_json::Value;

#[tokio::main]
async fn main() {
    // Parse command-line arguments using `clap`
    let args = match args::parse_args() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("Error: {}", err);
            return;
        }
    };

    // Extract arguments for convenience
    let db_host = args.db_url.clone();
    let table_name = args.table_name.clone();
    let dry_run = args.dry_run;
    let limit = args.limit;
    let script_dir = args.script_dir.clone();

    // Prepare Lua
    let lua = Arc::new(mlua::Lua::new());

    // load all include files
    for entry in fs::read_dir(script_dir.clone() + "/include").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_file() && path.extension() == Some("lua".as_ref()) {
            println!("include folder {:?}", path);

            let result = lua.load(path.clone()).exec();

            match result {
                Ok(()) => println!("Successfully loaded script {:?}", path),
                Err(err) => eprintln!("problem with {:?} - Error: {}", path, err),
            }
        }
    }

    // Validate that we have a valid lua script to transform the JSON input
    // A valid transformation requires proto file named with lua name
    // Example: Transaction.proto and Transaction.lua
    let lua_script = script_dir.clone() + "/" + &table_name + ".lua";
    if !fs::metadata(lua_script.clone()).is_ok() {
        eprintln!("Error: Lua script {:?} not found", lua_script);
        return;
    }

    println!("loading lua script {:?}", lua_script);
    let path = Path::new(&lua_script);
    let result = lua.load(path).exec();
    match result {
        Ok(()) => println!("Successfully loaded script {:?}", lua_script),
        Err(err) => {
            eprintln!("problem with {:?} - Error: {}", lua_script, err);
            return;
        }
    }

    // ensure that the lua script has a transform function
    let result: Result<mlua::Function, mlua::Error> = lua.globals().get("transform");
    match result {
        Ok(_) => println!(
            "Successfully loaded transform function from {:?}",
            lua_script
        ),
        Err(err) => {
            eprintln!("Error: transform function not found - {}", err);
            return;
        }
    }

    // Prepare protobuf
    // Parse the .proto file into a FileDescriptorSet
    let file_descriptor_set: FileDescriptorSet = Parser::new()
        .pure()
        .inputs(&[args.proto_path])
        .includes(&[args.proto_dir])
        .file_descriptor_set()
        .unwrap();
    let file_descriptor_set = Arc::new(file_descriptor_set);

    let fetcher = Fetch::new(&db_host, &table_name, limit);

    // fields to ignore because of couchdb metadata
    let ignore_list = vec!["_id".to_string(), "_rev".to_string()];

    fetcher
        .with_callback(Box::new({
            let file_descriptor_set: Arc<FileDescriptorSet> = Arc::clone(&file_descriptor_set);
            move |doc| {
                let err = valid_proto::validate_json(&file_descriptor_set, &table_name, &doc, ignore_list.clone());
                if err.len() > 0 {
                    // println!("{} will be updated because it does not match the schema", doc["_id"]);
                        let doc_clone = doc.clone();
                    let result = lua_transform(&lua.clone(), doc_clone);
                    match result {
                        Ok(transformed_doc) => {
                            // validate the transformed document again, if it is still invalid, return
                            let err = valid_proto::validate_json(&file_descriptor_set, &table_name, &transformed_doc, ignore_list.clone());
                            if err.len() > 0 {
                                println!();
                                println!("{} will not be updated because it still does not match the schema after transform", doc["_id"]);
                                for e in err {
                                    println!("Error: {} - {:?}", e.field, e.error_type);
                                }
                                println!("---------------------------------");
                                return;
                            } else if !dry_run {
                                let dbhost_clone = db_host.clone();
                                let table_name = table_name.clone();
                                tokio::spawn(async move {
                                    let client = Client::new();
                                    update_document(&client, &dbhost_clone, &table_name, &transformed_doc).await.unwrap();
                                    println!("{} updated successfully", doc["_id"]);
                                });
                            } else {
                                println!("{} will be updated", doc["_id"]);
                            }
                        }
                        Err(err) => {
                            eprintln!("Error: {}", err);
                            return;
                        },
                    }
                }
            }
        })) // closure to be called for each document
        .execute()
        .await;
}

// Execute transformation on the JSON input using the Lua script
fn lua_transform(lua: &Lua, doc: Value) -> Result<Value, Box<dyn std::error::Error>> {
    // Get the Lua transform method
    let transform: Function = lua.globals().get("transform")?;

    let input_json = doc.to_string();

    // Call the Lua function with the JSON input
    let output_str: String = transform.call(input_json)?;

    return serde_json::from_str(&output_str).map_err(|e| e.into());
}

/// Persists changes to a document in CouchDB when the dry-run mode is disabled.
async fn update_document(
    client: &Client,
    db_host: &str,
    table_name: &str,
    doc: &Value,
) -> Result<(), String> {
    let id = doc["_id"].as_str().ok_or("Document missing '_id' field")?;
    let rev = doc["_rev"]
        .as_str()
        .ok_or("Document missing '_rev' field")?;
    let idencoded = urlencoding::encode(id);
    let url = format!("{}/{}/{}", db_host, table_name, idencoded);

    let response = client
        .put(&url)
        .json(doc)
        .header("If-Match", rev)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() != StatusCode::OK && response.status() != StatusCode::CREATED {
        return Err(format!(
            "Failed to update document {}: Status code {}",
            id,
            response.status()
        ));
    }

    Ok(())
}
