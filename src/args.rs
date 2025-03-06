use clap::{Arg, Command};

pub struct Args {
    pub db_url: String,     // URL of the CouchDB database
    pub table_name: String, // Name of the table (or document type)
    pub ignore_list: String, // Comma-separated list of fields to ignore
    pub dry_run: bool, // Whether to perform a dry run (preview changes without modifying the database)
    pub stat: bool,    // Print list of document id without their error information
    pub limit: usize,  // Maximum number of documents to fetch per iteration
    pub proto_path: String, // Path to the .proto file
    pub proto_dir: String,  // Path containing .proto file
    pub script_dir: String, // Path to script that transform JSON document
}

/// Parse command-line arguments using `clap`
pub fn parse_args() -> Result<Args, String> {
    let name = env!("CARGO_PKG_NAME");
    let version = env!("CARGO_PKG_VERSION");
    let authors = env!("CARGO_PKG_AUTHORS");
    let description = env!("CARGO_PKG_DESCRIPTION");

    let matches = Command::new(name)
        .version(version)
        .author(authors)
        .about(description)
        .arg(
            Arg::new("db_prefix")
                .short('u')
                .long("url")
                .value_name("URL")
                .help("URL of the CouchDB database (Example: http://localhost:5984)")
                .required(true),
        )
        .arg(
            Arg::new("table_name")
                .short('t')
                .long("table")
                .value_name("TABLE")
                .help("Name of the table (or document type)")
                .required(true),
        )
        .arg(
            Arg::new("ignore")
                .short('g')
                .long("ignore")
                .value_name("IGNORE")
                .help("Comma-separated list of fields to ignore"),
        )
        .arg(
            Arg::new("proto")
                .short('p')
                .long("proto")
                .value_name("FILE")
                .help("Path to the .proto file")
                .required(true),
        )
        .arg(
            Arg::new("include")
                .short('i')
                .long("include")
                .value_name("DIRECTORY")
                .help("Path containing .proto file")
                .required(true),
        )
        .arg(
            Arg::new("dry_run")
                .long("dry-run") // Use --dry-run to enable dry-run mode
                .help("Enable dry-run mode (preview changes without modifying the database)")
                .action(clap::ArgAction::SetTrue) // Defaults to false unless --dry-run is provided
                .default_value("false"), // Default value is false (not dry-run)
        )
        .arg(
            Arg::new("stat") // Print list of document id without their error information.
                .long("stat")
                .help("Print list of document id without their error information.")
                .action(clap::ArgAction::SetTrue)
                .default_value("false"),
        )
        .arg(
            Arg::new("limit")
                .short('l')
                .long("limit")
                .value_name("LIMIT")
                .default_value("1000")
                .value_parser(clap::value_parser!(usize))
                .help("Maximum number of documents to fetch per iteration"),
        )
        .arg(
            Arg::new("luascript")
                .short('s')
                .long("script")
                .help("Path to script that transform JSON document"),
        )
        .get_matches();

    // Extract arguments from matches
    let db_url = matches.get_one::<String>("db_prefix").unwrap().clone();
    let table_name = matches.get_one::<String>("table_name").unwrap().clone();
    let ignore_list = matches.get_one::<String>("ignore").unwrap_or(&"".to_string()).clone();
    let dry_run = *matches.get_one::<bool>("dry_run").unwrap_or(&false);
    let stat = *matches.get_one::<bool>("stat").unwrap_or(&false);
    let limit = *matches.get_one::<usize>("limit").unwrap_or(&1000);
    // Read the .proto file
    let proto_path = matches.get_one::<String>("proto").unwrap().clone();
    let proto_dir = matches.get_one::<String>("include").unwrap().clone();

    let script_dir = matches
        .get_one::<String>("luascript")
        .unwrap_or(&"".to_string())
        .clone();

    Ok(Args {
        db_url,
        table_name,
        ignore_list,
        dry_run,
        stat,
        limit,
        proto_path,
        proto_dir,
        script_dir: script_dir,
    })
}
