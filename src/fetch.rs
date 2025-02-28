use reqwest::StatusCode;
use serde_json::{from_str, json, Value};

pub struct Fetch {
    dbprefix: String,
    dbtable: String,
    callback: Box<dyn Fn(Value) -> ()>,
    bookmark: Option<String>,
    limit: usize,
    doc_count: usize, // Total number of documents in the table
}

impl Fetch {
    pub fn new(dbprefix: &str, dbtable: &str, limit: usize) -> Self {
        Fetch {
            dbprefix: dbprefix.to_string(),
            dbtable: dbtable.to_string(),
            callback: Box::new(|_| ()),
            bookmark: None,
            limit,
            doc_count: 0,
        }
    }

    pub fn with_callback(mut self, callback: Box<dyn Fn(Value) -> ()>) -> Self {
        self.callback = callback;
        self
    }

    /// Executes the document fetching process.
    /// - Fetches metadata about the table.
    /// - Fetches documents in batches and applies the callback to each document.
    pub async fn execute(&mut self) {
        // Fetch metadata about the table (e.g., partitioned status, document count)
        match self.get_metadata().await {
            Ok(_) => (),
            Err(e) => {
                eprintln!("Failed to fetch table metadata: {}", e);
                return;
            }
        }

        let mut count = 1; // Counter for tracking the number of iterations
        let mut total_record = 0; // Total number of records fetched so far

        loop {
            // Fetch a batch of documents and apply the callback
            let num_of_record = self.fetch_and_apply().await.unwrap();
            total_record += num_of_record;

            // Log progress
            println!(
                "Fetched {}/{} transactions. Iteration: {}",
                total_record, self.doc_count, count
            );

            // Break the loop if fewer records than the limit are returned (end of data)
            if num_of_record < self.limit {
                break;
            }

            count += 1; // Increment the iteration counter
        }
    }

    async fn fetch_and_apply(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        let url = format!("{}/{}/_find", self.dbprefix, self.dbtable);

        let response = reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json")
            .body(self.selector())
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if response.status() != StatusCode::OK {
            return Err(format!(
                "Failed to fetch documents: Status code {}",
                response.status()
            )
            .into());
        }

        let body = response.text().await.map_err(|e| e.to_string())?;
        let json: Value = from_str(&body).map_err(|e| e.to_string())?;

        // Extract the bookmark for pagination
        self.bookmark = json
            .get("bookmark")
            .map(|b| b.as_str().unwrap())
            .map(|b| b.to_string());

        // Extract the "docs" array from the response
        let rows = json["docs"]
            .as_array()
            .ok_or("No 'docs' field in response")?;

        // Apply the callback to each document
        let count = rows
            .iter()
            .map(|doc| (self.callback)(doc.clone())) // Call the callback for each document
            .count(); // Count the number of documents processed

        Ok(count)
    }

    /// Fetches metadata about the table, including whether it is partitioned and the total document count.
    async fn get_metadata(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Construct the URL for fetching table metadata
        let url = format!("{}/{}", self.dbprefix, self.dbtable);

        let client = reqwest::Client::new();

        // Send a GET request to fetch metadata
        let response = client.get(&url).send().await.map_err(|e| e.to_string())?;

        // Check if the response status is successful (HTTP 200)
        if response.status() != StatusCode::OK {
            return Err(format!(
                "Failed to fetch table metadata: Status code {}",
                response.status()
            )
            .into());
        }

        // Parse the response body as JSON
        let body = response.text().await.map_err(|e| e.to_string())?;
        let json: Value = from_str(&body).map_err(|e| e.to_string())?;

        // Extract the total document count
        self.doc_count = json["doc_count"].as_u64().unwrap_or(0) as usize;

        Ok(())
    }

    /// Generates the JSON selector for querying transactions.
    fn selector(&self) -> String {
        let selector = SelectorContent {
            selector: json!({
                "_id": {
                    "$gt": null  // Transactions after the start date
                },
            }),
            limit: self.limit as i32, // Limit the number of records per query
            bookmark: self.bookmark.clone(), // Use the bookmark for pagination
        };

        // Serialize the selector to a JSON string
        serde_json::to_string(&selector).unwrap()
    }
}

/// Represents the structure of the query selector used for fetching documents.
#[derive(Debug, serde::Serialize)]
struct SelectorContent {
    selector: serde_json::Value, // JSON object representing the query conditions
    limit: i32,                  // Maximum number of records to fetch
    #[serde(skip_serializing_if = "Option::is_none")]
    bookmark: Option<String>, // Optional bookmark for pagination
}
