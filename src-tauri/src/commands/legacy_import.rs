use crate::db::Db;
use csv::ReaderBuilder;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tauri::State;

// Reads a backup produced by the pre-Recurring version of this app (subscriptions/occurrences,
// ledger-scoped budgets, flat tags) and converts it into the current schema. Two steps: preview
// computes an editable "migration plan" without touching the database - everything unambiguous
// gets a suggestion, everything else is flagged - then apply performs the real insert using
// whatever choices the frontend collected for the flagged items.

#[derive(Deserialize, Clone)]
struct OldAccountRow {
    id: i64,
    name: String,
    ledger: String,
    starting_balance_cents: i64,
    active: bool,
    is_default: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Deserialize, Clone)]
struct OldTagRow {
    id: i64,
    name: String,
    color: Option<String>,
}

#[derive(Deserialize)]
struct OldTransactionRow {
    id: i64,
    account_id: i64,
    date: String,
    amount_cents: i64,
    r#type: String,
    description: String,
    notes: Option<String>,
    linked_group_id: Option<String>,
    tag_ids: String,
    created_at: String,
    updated_at: String,
}

#[derive(Deserialize)]
struct OldSubscriptionRow {
    id: i64,
    account_id: i64,
    name: String,
    amount_cents: i64,
    r#type: String,
    interval_unit: String,
    interval_count: i64,
    start_date: String,
    #[allow(dead_code)]
    next_charge_date: String,
    end_date: Option<String>,
    active: bool,
    paused_until: Option<String>,
    notes: Option<String>,
    tag_ids: String,
    created_at: String,
    updated_at: String,
}

#[derive(Deserialize)]
struct OldOccurrenceRow {
    #[allow(dead_code)]
    id: i64,
    #[allow(dead_code)]
    subscription_id: i64,
    #[allow(dead_code)]
    due_date: String,
    #[allow(dead_code)]
    amount_cents: i64,
    #[allow(dead_code)]
    status: String,
    #[allow(dead_code)]
    paid_date: Option<String>,
}

#[derive(Deserialize)]
struct OldBudgetRow {
    id: i64,
    ledger: Option<String>,
    tag_id: Option<i64>,
    amount_cents: i64,
    show_on_dashboard: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, Clone)]
pub struct TagOption {
    pub id: i64,
    pub name: String,
}

#[derive(Serialize, Clone)]
pub struct AccountOption {
    pub id: i64,
    pub name: String,
}

#[derive(Serialize)]
pub struct SubscriptionResolution {
    pub subscription_id: i64,
    pub subscription_name: String,
    pub candidate_tags: Vec<TagOption>,
    pub suggested_tag_id: Option<i64>,
    pub suggested_new_tag_name: String,
    pub needs_review: bool,
    pub dropped_fields_note: Option<String>,
}

#[derive(Serialize)]
pub struct BudgetResolution {
    pub budget_id: i64,
    pub ledger: Option<String>,
    pub matching_accounts: Vec<AccountOption>,
    pub suggested_account_id: Option<i64>,
    pub needs_review: bool,
}

#[derive(Serialize)]
pub struct LegacyMigrationPlan {
    pub accounts: usize,
    pub tags: usize,
    pub transactions: usize,
    pub occurrences_dropped: usize,
    pub all_tags: Vec<TagOption>,
    pub all_accounts: Vec<AccountOption>,
    pub subscription_resolutions: Vec<SubscriptionResolution>,
    pub budget_resolutions: Vec<BudgetResolution>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionChoice {
    pub subscription_id: i64,
    pub tag_id: Option<i64>,
    pub new_tag_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetChoice {
    pub budget_id: i64,
    pub account_id: Option<i64>,
}

#[derive(Serialize)]
pub struct LegacyImportSummary {
    pub accounts: usize,
    pub tags: usize,
    pub transactions: usize,
    pub recurring: usize,
    pub budgets: usize,
    pub occurrences_dropped: usize,
}

#[tauri::command]
pub fn preview_legacy_import(dir: String) -> Result<LegacyMigrationPlan, String> {
    let dir = Path::new(&dir);
    let accounts: Vec<OldAccountRow> = read_csv(dir.join("accounts.csv"))?;
    let tags: Vec<OldTagRow> = read_csv(dir.join("tags.csv"))?;
    let transactions: Vec<OldTransactionRow> = read_csv(dir.join("transactions.csv"))?;
    let subscriptions: Vec<OldSubscriptionRow> = read_csv(dir.join("subscriptions.csv"))?;
    let occurrences: Vec<OldOccurrenceRow> = read_csv(dir.join("subscription_occurrences.csv"))?;
    let budgets: Vec<OldBudgetRow> = read_csv(dir.join("budgets.csv"))?;

    let tag_by_id: HashMap<i64, OldTagRow> = tags.iter().map(|t| (t.id, t.clone())).collect();
    let account_by_id: HashMap<i64, OldAccountRow> = accounts.iter().map(|a| (a.id, a.clone())).collect();

    let mut ledger_to_accounts: HashMap<String, Vec<i64>> = HashMap::new();
    for a in &accounts {
        ledger_to_accounts.entry(a.ledger.clone()).or_default().push(a.id);
    }

    // A subscription could carry zero, one, or several tags (many-to-many), but a Recurring
    // entry needs exactly one, and it can't already belong to another Recurring - so track which
    // tags earlier subscriptions in this same batch have already claimed.
    let mut claimed_tags: HashSet<i64> = HashSet::new();
    let mut subscription_resolutions = Vec::with_capacity(subscriptions.len());
    for sub in &subscriptions {
        let tag_ids = parse_ids(&sub.tag_ids);
        let candidate_tags: Vec<TagOption> = tag_ids
            .iter()
            .filter_map(|id| tag_by_id.get(id))
            .map(|t| TagOption { id: t.id, name: t.name.clone() })
            .collect();
        let available: Vec<i64> = tag_ids.iter().copied().filter(|id| !claimed_tags.contains(id)).collect();
        let (suggested_tag_id, needs_review) = if tag_ids.is_empty() {
            (None, false)
        } else if tag_ids.len() == 1 && available.len() == 1 {
            (Some(available[0]), false)
        } else {
            (available.first().copied(), true)
        };
        if let Some(t) = suggested_tag_id {
            claimed_tags.insert(t);
        }
        subscription_resolutions.push(SubscriptionResolution {
            subscription_id: sub.id,
            subscription_name: sub.name.clone(),
            candidate_tags,
            suggested_tag_id,
            suggested_new_tag_name: sub.name.clone(),
            needs_review,
            dropped_fields_note: dropped_fields_note(sub),
        });
    }

    let mut budget_resolutions = Vec::with_capacity(budgets.len());
    for b in &budgets {
        let (matching_accounts, suggested_account_id, needs_review) = match &b.ledger {
            None => (vec![], None, false),
            Some(ledger) => {
                let ids = ledger_to_accounts.get(ledger).cloned().unwrap_or_default();
                let opts: Vec<AccountOption> = ids
                    .iter()
                    .filter_map(|id| account_by_id.get(id))
                    .map(|a| AccountOption { id: a.id, name: a.name.clone() })
                    .collect();
                let suggested = if ids.len() == 1 { Some(ids[0]) } else { ids.first().copied() };
                (opts, suggested, ids.len() != 1)
            }
        };
        budget_resolutions.push(BudgetResolution {
            budget_id: b.id,
            ledger: b.ledger.clone(),
            matching_accounts,
            suggested_account_id,
            needs_review,
        });
    }

    Ok(LegacyMigrationPlan {
        accounts: accounts.len(),
        tags: tags.len(),
        transactions: transactions.len(),
        occurrences_dropped: occurrences.len(),
        all_tags: tags.iter().map(|t| TagOption { id: t.id, name: t.name.clone() }).collect(),
        all_accounts: accounts.iter().map(|a| AccountOption { id: a.id, name: a.name.clone() }).collect(),
        subscription_resolutions,
        budget_resolutions,
    })
}

/// Applies a legacy backup using the frontend's resolved choices - one entry per subscription
/// and per budget, since the frontend already has the full plan from `preview_legacy_import` and
/// carries each item's suggested default forward unless the user changed it. Wrapped in one
/// transaction, same all-or-nothing guarantee as the regular restore, and likewise meant for an
/// empty database - an id collision fails the whole import rather than duplicating anything.
#[tauri::command]
pub fn apply_legacy_import(
    db: State<Db>,
    dir: String,
    subscription_choices: Vec<SubscriptionChoice>,
    budget_choices: Vec<BudgetChoice>,
) -> Result<LegacyImportSummary, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    apply_legacy_import_inner(&mut conn, Path::new(&dir), &subscription_choices, &budget_choices)
}

fn apply_legacy_import_inner(
    conn: &mut Connection,
    dir: &Path,
    subscription_choices: &[SubscriptionChoice],
    budget_choices: &[BudgetChoice],
) -> Result<LegacyImportSummary, String> {
    let accounts: Vec<OldAccountRow> = read_csv(dir.join("accounts.csv"))?;
    let tags: Vec<OldTagRow> = read_csv(dir.join("tags.csv"))?;
    let transactions: Vec<OldTransactionRow> = read_csv(dir.join("transactions.csv"))?;
    let subscriptions: Vec<OldSubscriptionRow> = read_csv(dir.join("subscriptions.csv"))?;
    let occurrences: Vec<OldOccurrenceRow> = read_csv(dir.join("subscription_occurrences.csv"))?;
    let budgets: Vec<OldBudgetRow> = read_csv(dir.join("budgets.csv"))?;

    let sub_choice_by_id: HashMap<i64, &SubscriptionChoice> =
        subscription_choices.iter().map(|c| (c.subscription_id, c)).collect();
    let budget_choice_by_id: HashMap<i64, &BudgetChoice> =
        budget_choices.iter().map(|c| (c.budget_id, c)).collect();

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    crate::db::clear_seed_tags_for_fresh_import(&tx)?;

    for a in &accounts {
        tx.execute(
            "INSERT INTO accounts (id, name, ledger, starting_balance_cents, active, is_default, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![a.id, a.name, a.ledger, a.starting_balance_cents, a.active, a.is_default, a.created_at, a.updated_at],
        ).map_err(|e| e.to_string())?;
    }

    for t in &tags {
        tx.execute(
            "INSERT INTO tags (id, name, color, parent_id) VALUES (?1, ?2, ?3, NULL)",
            params![t.id, t.name, t.color],
        )
        .map_err(|e| e.to_string())?;
    }

    for t in &transactions {
        tx.execute(
            "INSERT INTO transactions (id, account_id, date, amount_cents, type, description, notes, linked_group_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![t.id, t.account_id, t.date, t.amount_cents, t.r#type, t.description, t.notes, t.linked_group_id, t.created_at, t.updated_at],
        ).map_err(|e| e.to_string())?;
        for tag_id in parse_ids(&t.tag_ids) {
            tx.execute(
                "INSERT INTO transaction_tags (transaction_id, tag_id) VALUES (?1, ?2)",
                params![t.id, tag_id],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    let mut tags_created = 0usize;
    for sub in &subscriptions {
        let choice = sub_choice_by_id
            .get(&sub.id)
            .ok_or_else(|| format!("Missing a resolution for subscription #{} ({})", sub.id, sub.name))?;
        let tag_id = match (choice.tag_id, &choice.new_tag_name) {
            (Some(id), _) => id,
            (None, Some(name)) => {
                tx.execute(
                    "INSERT INTO tags (name, color, parent_id) VALUES (?1, NULL, NULL)",
                    params![name],
                )
                .map_err(|e| e.to_string())?;
                tags_created += 1;
                tx.last_insert_rowid()
            }
            (None, None) => {
                return Err(format!(
                    "Subscription #{} ({}) needs either an existing tag or a new tag name",
                    sub.id, sub.name
                ))
            }
        };
        let dropped = dropped_fields_note(sub);
        let notes = match (&sub.notes, &dropped) {
            (notes, None) => notes.clone(),
            (Some(existing), Some(note)) => Some(format!("{existing} [migrated: {note}]")),
            (None, Some(note)) => Some(format!("[migrated: {note}]")),
        };
        tx.execute(
            "INSERT INTO recurring (id, tag_id, account_id, type, interval_unit, interval_count, anchor_date, projected_amount_cents, notes, active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![sub.id, tag_id, sub.account_id, sub.r#type, sub.interval_unit, sub.interval_count, sub.start_date, sub.amount_cents, notes, sub.active, sub.created_at, sub.updated_at],
        ).map_err(|e| e.to_string())?;
    }

    for b in &budgets {
        let account_id = budget_choice_by_id.get(&b.id).and_then(|c| c.account_id);
        tx.execute(
            "INSERT INTO budgets (id, account_id, tag_id, amount_cents, show_on_dashboard, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![b.id, account_id, b.tag_id, b.amount_cents, b.show_on_dashboard, b.created_at, b.updated_at],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;

    Ok(LegacyImportSummary {
        accounts: accounts.len(),
        tags: tags.len() + tags_created,
        transactions: transactions.len(),
        recurring: subscriptions.len(),
        budgets: budgets.len(),
        occurrences_dropped: occurrences.len(),
    })
}

fn dropped_fields_note(sub: &OldSubscriptionRow) -> Option<String> {
    let mut dropped: Vec<&str> = Vec::new();
    if sub.paused_until.is_some() {
        dropped.push("was paused");
    }
    if sub.end_date.is_some() {
        dropped.push("had an end date");
    }
    if dropped.is_empty() {
        None
    } else {
        Some(dropped.join(", "))
    }
}

fn parse_ids(s: &str) -> Vec<i64> {
    s.split('|').filter(|p| !p.is_empty()).filter_map(|p| p.parse().ok()).collect()
}

fn read_csv<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Result<Vec<T>, String> {
    let mut rdr = ReaderBuilder::new()
        .from_path(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    rdr.deserialize::<T>()
        .collect::<Result<Vec<T>, _>>()
        .map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_in_memory_for_test;

    /// Writes a synthetic pre-Recurring backup exercising every resolution case at once:
    /// - a subscription with one tag (auto-resolves, no review)
    /// - a subscription with two tags where one is already claimed by another subscription
    ///   (forces review; the test deliberately overrides the suggestion)
    /// - two untagged subscriptions, one of them inactive with an end date (both auto-create a
    ///   new tag from their name; the inactive flag maps straight to the new active column, and
    ///   the end date - which has no Recurring equivalent - gets a "[migrated: ...]" notes suffix)
    /// - a budget whose ledger matches two accounts (forces review; overridden in the test)
    /// - a budget whose ledger matches exactly one account (auto-resolves)
    /// - a budget with no ledger at all (already "any account", no review)
    fn write_fixture(dir: &std::path::Path) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join("accounts.csv"),
            "id,name,ledger,starting_balance_cents,active,is_default,created_at,updated_at\n\
             1,Checking,personal,100000,true,true,2024-01-01T00:00:00,2024-01-01T00:00:00\n\
             2,Savings,personal,500000,true,false,2024-01-01T00:00:00,2024-01-01T00:00:00\n\
             3,Business Checking,business,20000,true,false,2024-01-01T00:00:00,2024-01-01T00:00:00\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("tags.csv"),
            "id,name,color\n\
             1,Groceries,#3987e5\n\
             2,Rent,#3987e5\n\
             3,Netflix,#3987e5\n\
             4,Streaming,#3987e5\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("transactions.csv"),
            "id,account_id,date,amount_cents,type,description,notes,linked_group_id,tag_ids,created_at,updated_at\n\
             1,1,2024-01-05,-5000,expense,Groceries run,,,1,2024-01-05T00:00:00,2024-01-05T00:00:00\n\
             2,1,2024-01-10,-1500,expense,Netflix charge,,,3,2024-01-10T00:00:00,2024-01-10T00:00:00\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("subscriptions.csv"),
            "id,account_id,name,amount_cents,type,interval_unit,interval_count,start_date,next_charge_date,end_date,active,paused_until,notes,tag_ids,created_at,updated_at\n\
             1,1,Netflix,1500,expense,month,1,2024-01-01,2024-02-01,,true,,,3,2024-01-01T00:00:00,2024-01-01T00:00:00\n\
             2,1,Spotify,999,expense,month,1,2024-01-01,2024-02-01,,true,,,3|4,2024-01-01T00:00:00,2024-01-01T00:00:00\n\
             3,1,Gym Membership,4000,expense,month,1,2024-01-01,2024-02-01,,true,,,,2024-01-01T00:00:00,2024-01-01T00:00:00\n\
             4,1,Old Unused Sub,999,expense,month,1,2023-01-01,2023-02-01,2023-06-01,false,,,,2023-01-01T00:00:00,2023-06-01T00:00:00\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("subscription_occurrences.csv"),
            "id,subscription_id,due_date,amount_cents,status,paid_date\n\
             1,1,2024-01-01,1500,confirmed,2024-01-01\n\
             2,2,2024-01-01,999,confirmed,2024-01-02\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("budgets.csv"),
            "id,ledger,tag_id,amount_cents,show_on_dashboard,created_at,updated_at\n\
             1,personal,1,50000,true,2024-01-01T00:00:00,2024-01-01T00:00:00\n\
             2,business,2,20000,true,2024-01-01T00:00:00,2024-01-01T00:00:00\n\
             3,,,10000,false,2024-01-01T00:00:00,2024-01-01T00:00:00\n",
        )
        .unwrap();
    }

    struct TempDir(std::path::PathBuf);
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn preview_then_apply_resolves_and_imports_correctly() {
        let dir = TempDir(std::env::temp_dir().join(format!("legacy_import_test_{}", std::process::id())));
        write_fixture(&dir.0);
        let dir_str = dir.0.to_string_lossy().to_string();

        let plan = preview_legacy_import(dir_str.clone()).expect("preview should succeed");

        assert_eq!(plan.accounts, 3);
        assert_eq!(plan.tags, 4);
        assert_eq!(plan.transactions, 2);
        assert_eq!(plan.occurrences_dropped, 2);
        assert_eq!(plan.subscription_resolutions.len(), 4);
        assert_eq!(plan.budget_resolutions.len(), 3);

        let sub = |id: i64| plan.subscription_resolutions.iter().find(|r| r.subscription_id == id).unwrap();
        let netflix = sub(1);
        assert!(!netflix.needs_review);
        assert_eq!(netflix.suggested_tag_id, Some(3));

        let spotify = sub(2);
        assert!(spotify.needs_review, "two original tags should always force a review");
        assert_eq!(spotify.suggested_tag_id, Some(4), "tag 3 was already claimed by Netflix");

        let gym = sub(3);
        assert!(!gym.needs_review);
        assert_eq!(gym.suggested_tag_id, None);
        assert_eq!(gym.suggested_new_tag_name, "Gym Membership");
        assert!(gym.dropped_fields_note.is_none());

        let old_unused = sub(4);
        assert!(!old_unused.needs_review);
        assert_eq!(old_unused.suggested_tag_id, None);
        let note = old_unused.dropped_fields_note.as_deref().unwrap();
        assert!(note.contains("had an end date"), "an end date has no Recurring equivalent, so it's still noted");
        assert!(!note.contains("was paused"));

        let budget = |id: i64| plan.budget_resolutions.iter().find(|r| r.budget_id == id).unwrap();
        let personal_budget = budget(1);
        assert!(personal_budget.needs_review, "ledger shared by two accounts should force a review");
        assert_eq!(personal_budget.matching_accounts.len(), 2);
        let business_budget = budget(2);
        assert!(!business_budget.needs_review);
        assert_eq!(business_budget.suggested_account_id, Some(3));
        let overall_budget = budget(3);
        assert!(!overall_budget.needs_review);
        assert_eq!(overall_budget.suggested_account_id, None);

        // Carry every suggestion forward as-is, except two deliberate overrides (Spotify's tag,
        // and the personal budget's account) to prove a reviewed choice actually takes effect
        // instead of the auto-suggestion.
        let subscription_choices: Vec<SubscriptionChoice> = plan
            .subscription_resolutions
            .iter()
            .map(|r| {
                if r.subscription_id == 2 {
                    SubscriptionChoice { subscription_id: 2, tag_id: Some(2), new_tag_name: None }
                } else {
                    SubscriptionChoice {
                        subscription_id: r.subscription_id,
                        tag_id: r.suggested_tag_id,
                        new_tag_name: if r.suggested_tag_id.is_none() { Some(r.suggested_new_tag_name.clone()) } else { None },
                    }
                }
            })
            .collect();
        let budget_choices: Vec<BudgetChoice> = plan
            .budget_resolutions
            .iter()
            .map(|r| {
                if r.budget_id == 1 {
                    BudgetChoice { budget_id: 1, account_id: Some(2) }
                } else {
                    BudgetChoice { budget_id: r.budget_id, account_id: r.suggested_account_id }
                }
            })
            .collect();

        let mut conn = init_in_memory_for_test();
        let summary = apply_legacy_import_inner(&mut conn, &dir.0, &subscription_choices, &budget_choices)
            .expect("apply should succeed");

        assert_eq!(summary.accounts, 3);
        assert_eq!(summary.tags, 6, "4 original tags + Gym Membership + Old Unused Sub");
        assert_eq!(summary.transactions, 2);
        assert_eq!(summary.recurring, 4);
        assert_eq!(summary.budgets, 3);
        assert_eq!(summary.occurrences_dropped, 2);

        let count = |sql: &str| -> i64 { conn.query_row(sql, [], |r| r.get(0)).unwrap() };
        assert_eq!(count("SELECT COUNT(*) FROM accounts"), 3);
        assert_eq!(count("SELECT COUNT(*) FROM tags"), 6);
        assert_eq!(count("SELECT COUNT(*) FROM transactions"), 2);
        assert_eq!(count("SELECT COUNT(*) FROM transaction_tags"), 2);
        assert_eq!(count("SELECT COUNT(*) FROM recurring"), 4);
        assert_eq!(count("SELECT COUNT(*) FROM budgets"), 3);

        let netflix_tag_id: i64 = conn
            .query_row("SELECT tag_id FROM recurring WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(netflix_tag_id, 3);

        let spotify_tag_id: i64 = conn
            .query_row("SELECT tag_id FROM recurring WHERE id = 2", [], |r| r.get(0))
            .unwrap();
        assert_eq!(spotify_tag_id, 2, "override should have won over the auto-suggested tag 4");

        let gym_tag_id: i64 = conn
            .query_row("SELECT id FROM tags WHERE name = 'Gym Membership'", [], |r| r.get(0))
            .unwrap();
        let gym_recurring_tag_id: i64 = conn
            .query_row("SELECT tag_id FROM recurring WHERE id = 3", [], |r| r.get(0))
            .unwrap();
        assert_eq!(gym_recurring_tag_id, gym_tag_id);

        let (old_unused_tag_id, old_unused_notes, old_unused_active): (i64, Option<String>, i64) = conn
            .query_row("SELECT tag_id, notes, active FROM recurring WHERE id = 4", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .unwrap();
        assert_ne!(old_unused_tag_id, gym_tag_id, "each new tag must be distinct");
        assert_eq!(old_unused_active, 0, "the old subscription's active=false should map directly to the new column");
        let notes = old_unused_notes.unwrap();
        assert!(notes.contains("had an end date"));

        let personal_budget_account: Option<i64> = conn
            .query_row("SELECT account_id FROM budgets WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(personal_budget_account, Some(2), "override should have won over the auto-suggested account 1");

        let business_budget_account: Option<i64> = conn
            .query_row("SELECT account_id FROM budgets WHERE id = 2", [], |r| r.get(0))
            .unwrap();
        assert_eq!(business_budget_account, Some(3));

        let overall_budget_account: Option<i64> = conn
            .query_row("SELECT account_id FROM budgets WHERE id = 3", [], |r| r.get(0))
            .unwrap();
        assert_eq!(overall_budget_account, None);

        // All four recurring tag_ids must be distinct, or the UNIQUE constraint would already
        // have failed the insert above - double-checking explicitly for a clearer failure message.
        let mut stmt = conn.prepare("SELECT tag_id FROM recurring").unwrap();
        let tag_ids: Vec<i64> = stmt.query_map([], |r| r.get(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap();
        let unique: std::collections::HashSet<i64> = tag_ids.iter().copied().collect();
        assert_eq!(tag_ids.len(), unique.len());
    }
}
