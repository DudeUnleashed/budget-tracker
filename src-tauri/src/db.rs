use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};
use std::path::Path;
use std::sync::Mutex;

pub struct Db(pub Mutex<Connection>);

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(include_str!("../migrations/0001_initial.sql")),
        M::up(include_str!("../migrations/0002_budget_dashboard_toggle.sql")),
        M::up(include_str!("../migrations/0003_account_default.sql")),
    ])
}

pub fn init(app_data_dir: &Path) -> Connection {
    std::fs::create_dir_all(app_data_dir).expect("failed to create app data dir");
    let db_path = app_data_dir.join("budget-tracker.db");
    let mut conn = Connection::open(db_path).expect("failed to open database");
    conn.pragma_update(None, "foreign_keys", true)
        .expect("failed to enable foreign keys");
    migrations()
        .to_latest(&mut conn)
        .expect("failed to run migrations");
    conn
}
