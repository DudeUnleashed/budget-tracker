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
        M::up(include_str!("../migrations/0004_recurring_and_tag_hierarchy.sql")),
        M::up(include_str!("../migrations/0005_budget_month_overrides.sql")),
        M::up(include_str!("../migrations/0006_budget_account_scope.sql")),
        M::up(include_str!("../migrations/0007_remove_seed_tags.sql")),
        M::up(include_str!("../migrations/0008_debts.sql")),
        M::up(include_str!("../migrations/0009_recurring_active.sql")),
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
    relax_tag_name_uniqueness(&mut conn);
    conn
}

const SEED_TAG_NAMES: [&str; 3] = ["Owner's Draw", "Business Loan", "Loan Repayment"];

/// Installs that predate migration 0007 may still have three built-in cross-ledger tags that
/// used to be auto-seeded, so a database from one of those isn't necessarily as empty as it
/// looks even before a user enters anything. Both restore paths (the regular CSV restore and the
/// legacy-version converter) are documented as only supporting a fresh install - but taken
/// literally, that meant a real restore onto one of those not-quite-empty installs failed
/// immediately on an id collision with those leftover seeded rows, since every version of this
/// app has always exported them like any other tag. This checks nothing but the (now-optional)
/// seed tags exists yet and, if so, clears them so the incoming backup's own copies can be
/// inserted cleanly. If anything else is already present, it refuses rather than silently
/// deleting real data. On an install created after 0007, this is just a no-op.
pub fn clear_seed_tags_for_fresh_import(conn: &Connection) -> Result<(), String> {
    for (table, label) in [
        ("accounts", "accounts"),
        ("transactions", "transactions"),
        ("recurring", "recurring items"),
        ("budgets", "budgets"),
    ] {
        let count: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if count > 0 {
            return Err(format!(
                "This database already has {label} in it - restoring a backup is only supported into a fresh install."
            ));
        }
    }
    let placeholders = SEED_TAG_NAMES.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let non_seed_count: i64 = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM tags WHERE name NOT IN ({placeholders})"),
            rusqlite::params_from_iter(SEED_TAG_NAMES.iter()),
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if non_seed_count > 0 {
        return Err(
            "This database already has categories in it - restoring a backup is only supported into a fresh install."
                .into(),
        );
    }
    conn.execute("DELETE FROM tags", []).map_err(|e| e.to_string())?;
    Ok(())
}

/// Same schema setup as `init`, against a throwaway in-memory database - for tests that need a
/// real current-schema connection (foreign keys, the recurring.tag_id UNIQUE constraint, etc.)
/// without touching disk.
#[cfg(test)]
pub(crate) fn init_in_memory_for_test() -> Connection {
    let mut conn = Connection::open_in_memory().expect("failed to open in-memory database");
    conn.pragma_update(None, "foreign_keys", true)
        .expect("failed to enable foreign keys");
    migrations()
        .to_latest(&mut conn)
        .expect("failed to run migrations");
    relax_tag_name_uniqueness(&mut conn);
    conn
}

/// Tag names only need to be unique among siblings (same parent), not globally - e.g. a
/// top-level "Subscriptions" category and a "Subscriptions" tag nested under Groceries should
/// both be allowed. The original schema baked a single global UNIQUE(name) into the table
/// definition, and SQLite can only remove a column-level constraint by rebuilding the table.
/// That rebuild has to run with `foreign_keys` off - PRAGMA foreign_keys is a documented no-op
/// once a transaction is open, and rusqlite_migration always wraps each numbered migration in
/// one - so unlike everything else, this can't live in migrations/*.sql. It runs once after
/// every migration pass and is a no-op thereafter, once the sibling-scoped indexes already exist.
fn relax_tag_name_uniqueness(conn: &mut Connection) {
    let already_done: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_tags_name_sibling'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;
    if already_done {
        return;
    }
    conn.pragma_update(None, "foreign_keys", false)
        .expect("failed to disable foreign keys for tag table rebuild");
    let tx = conn.transaction().expect("failed to start tag rebuild transaction");
    tx.execute_batch(
        "CREATE TABLE tags_new (
            id        INTEGER PRIMARY KEY,
            name      TEXT NOT NULL,
            color     TEXT,
            parent_id INTEGER REFERENCES tags(id) ON DELETE SET NULL
         );
         INSERT INTO tags_new (id, name, color, parent_id) SELECT id, name, color, parent_id FROM tags;
         DROP TABLE tags;
         ALTER TABLE tags_new RENAME TO tags;
         CREATE INDEX idx_tags_parent ON tags(parent_id);
         CREATE UNIQUE INDEX idx_tags_name_sibling ON tags(parent_id, name) WHERE parent_id IS NOT NULL;
         CREATE UNIQUE INDEX idx_tags_name_top_level ON tags(name) WHERE parent_id IS NULL;",
    )
    .expect("failed to rebuild tags table");
    tx.commit().expect("failed to commit tag table rebuild");
    conn.pragma_update(None, "foreign_keys", true)
        .expect("failed to re-enable foreign keys after tag rebuild");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simulates a pre-0007 install: migrate up through 0006 only, then manually seed the three
    /// built-in cross-ledger tags the way migration 0001 used to, exactly as a real upgrading
    /// database would have them. One of the three is actually tagged on a real transaction; the
    /// other two are untouched. Migration 0007 should then delete only the two unused ones and
    /// leave the in-use one (and its real transaction association) completely alone.
    #[test]
    fn migration_0007_removes_only_unused_seed_tags() {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        conn.pragma_update(None, "foreign_keys", true).expect("enable foreign keys");
        migrations().to_version(&mut conn, 6).expect("migrate through 0006");

        conn.execute_batch(
            "INSERT INTO tags (id, name, color) VALUES
                (1, 'Owner''s Draw', '#22c55e'),
                (2, 'Business Loan', '#f59e0b'),
                (3, 'Loan Repayment', '#3b82f6');
             INSERT INTO accounts (id, name, ledger, starting_balance_cents, active, is_default, created_at, updated_at)
                VALUES (1, 'Checking', 'personal', 0, 1, 1, '2024-01-01T00:00:00', '2024-01-01T00:00:00');
             INSERT INTO transactions (id, account_id, date, amount_cents, type, description, created_at, updated_at)
                VALUES (1, 1, '2024-01-01', -1000, 'expense', 'Loan payment', '2024-01-01T00:00:00', '2024-01-01T00:00:00');
             INSERT INTO transaction_tags (transaction_id, tag_id) VALUES (1, 2);",
        )
        .expect("seed pre-0007 state");

        migrations().to_version(&mut conn, 7).expect("migrate to 0007");

        let remaining: Vec<String> = {
            let mut stmt = conn.prepare("SELECT name FROM tags ORDER BY name").unwrap();
            stmt.query_map([], |r| r.get(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap()
        };
        assert_eq!(remaining, vec!["Business Loan".to_string()], "only the in-use seed tag should survive");

        let still_tagged: i64 = conn
            .query_row("SELECT COUNT(*) FROM transaction_tags WHERE transaction_id = 1 AND tag_id = 2", [], |r| r.get(0))
            .unwrap();
        assert_eq!(still_tagged, 1, "the real transaction's tag association must not be cascaded away");
    }

    #[test]
    fn fresh_install_never_gets_seed_tags() {
        let conn = init_in_memory_for_test();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM tags", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0, "a brand-new database should start with zero tags");
    }
}
