use eframe::egui;
use tokio_postgres::{NoTls, Error as PgError};
use futures::executor::block_on;
use std::fs;
use std::io::{BufRead, BufReader};
use std::collections::HashMap;

struct MyApp {
    customers: Vec<Customer>,
    current_index: usize,
    pool: tokio_postgres::Client,
}

struct Customer {
    fields: Vec<(String, String)>,
}

impl MyApp {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = "/home/alex/.config/db_crm_cred.ini";
        let file = fs::File::open(config_path)?;
        let reader = BufReader::new(file);
        let mut db_config = HashMap::new();
        for line in reader.lines() {
            let line = line?;
            if line.starts_with('[') || line.trim().is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                db_config.insert(parts[0].trim().to_string(), parts[1].trim().to_string());
            }
        }

        let connection_string = format!(
            "host={} user={} password={} dbname={}",
            db_config.get("host").unwrap_or(&"localhost".to_string()),
            db_config.get("user").unwrap_or(&"".to_string()),
            db_config.get("password").unwrap_or(&"".to_string()),
            db_config.get("dbname").unwrap_or(&"".to_string())
        );

        let (client, connection) = tokio_postgres::connect(&connection_string, NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        let mut app = MyApp {
            customers: Vec::new(),
            current_index: 0,
            pool: client,
        };

        app.fetch_customers().await?;

        Ok(app)
    }

    async fn fetch_customers(&mut self) -> Result<(), PgError> {
        // Check if the table exists
        let table_exists = self.pool.query_one("SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'customers')", &[]).await?;
        let exists: bool = table_exists.get(0);
        println!("Customers table exists: {}", exists);

        if !exists {
            println!("The 'customers' table does not exist.");
            return Ok(());
        }

        // Count the number of records
        let count_result = self.pool.query_one("SELECT COUNT(*) FROM customers", &[]).await?;
        let count: i64 = count_result.get(0);
        println!("Number of records in customers table: {}", count);

        // Fetch all customers
        let rows = self.pool.query("SELECT * FROM customers", &[]).await?;
    
        // Print table headers
        if let Some(first_row) = rows.first() {
            println!("Table headers:");
            for column in first_row.columns() {
                print!("{}\t", column.name());
            }
            println!();
        }

        // Print all data
        println!("Table data:");
        for row in &rows {
            for (i, column) in row.columns().iter().enumerate() {
                let value: String = match column.type_().name() {
                    "int4" => row.get::<_, i32>(i).to_string(),
                    _ => row.get::<_, String>(i),
                };
                print!("{}\t", value);
            }
            println!();
        }

        self.customers = rows.into_iter().map(|row| {
            let mut fields = Vec::new();
            for (i, column) in row.columns().iter().enumerate() {
                let value = match column.type_().name() {
                    "int4" => row.get::<_, i32>(i).to_string(),
                    _ => row.get::<_, String>(i),
                };
                fields.push((column.name().to_string(), value));
            }
            Customer { fields }
        }).collect();

        Ok(())
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Kunden-Formular");

            if let Some(customer) = self.customers.get(self.current_index) {
                for (field_name, field_value) in &customer.fields {
                    ui.horizontal(|ui| {
                        let capitalized_label = field_name.chars().next().unwrap().to_uppercase().collect::<String>() + &field_name[1..];
                        ui.label(&capitalized_label);
                        let mut value = field_value.to_string();
                        ui.text_edit_singleline(&mut value);
                    });
                }
            }

            ui.horizontal(|ui| {
                if ui.button("Vorheriger").clicked() && self.current_index > 0 {
                    self.current_index -= 1;
                }
                if ui.button("Nächster").clicked() && self.current_index < self.customers.len() - 1 {
                    self.current_index += 1;
                }
            });
        });
    }
}

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    let app = MyApp::new().await.expect("Failed to initialize app");

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 480.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Kunden-App",
        options,
        Box::new(|_cc| Box::new(app)),
    )
}
