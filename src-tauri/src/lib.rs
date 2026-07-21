mod commands;
mod db;
mod debt_math;
mod models;
mod recurrence;

use commands::accounts::{
    create_account, delete_account, get_account_balance, list_accounts, set_default_account, update_account,
};
use commands::budgets::{
    clear_budget_month_override, create_budget, delete_budget, get_budget_breakdown, get_budget_progress,
    get_tag_breakdown, list_budgets, set_budget_month_override, update_budget,
};
use commands::dashboard::{get_month_summary, get_monthly_trend};
use commands::data::{export_data, import_data, wipe_all_data};
use commands::debts::{create_debt, delete_debt, list_debts, preview_debt_payoff, update_debt};
use commands::legacy_import::{apply_legacy_import, preview_legacy_import};
use commands::recurring::{
    create_recurring, delete_recurring, list_recurring, list_recurring_cycle_transactions, set_recurring_active,
    update_recurring,
};
use commands::tags::{create_tag, delete_tag, list_tags, update_tag};
use commands::transactions::{
    create_account_transfer, create_transaction, delete_transaction, list_transactions_for_month,
    list_transactions_in_range, search_transactions, update_transaction,
};
use db::Db;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            let conn = db::init(&app_data_dir);
            app.manage(Db(Mutex::new(conn)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_account,
            list_accounts,
            update_account,
            delete_account,
            set_default_account,
            get_account_balance,
            create_tag,
            list_tags,
            update_tag,
            delete_tag,
            create_transaction,
            create_account_transfer,
            update_transaction,
            delete_transaction,
            list_transactions_for_month,
            list_transactions_in_range,
            search_transactions,
            create_recurring,
            update_recurring,
            delete_recurring,
            list_recurring,
            list_recurring_cycle_transactions,
            set_recurring_active,
            create_budget,
            list_budgets,
            update_budget,
            delete_budget,
            get_budget_progress,
            get_budget_breakdown,
            get_tag_breakdown,
            set_budget_month_override,
            clear_budget_month_override,
            get_month_summary,
            get_monthly_trend,
            export_data,
            import_data,
            wipe_all_data,
            preview_legacy_import,
            apply_legacy_import,
            create_debt,
            update_debt,
            delete_debt,
            list_debts,
            preview_debt_payoff,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
