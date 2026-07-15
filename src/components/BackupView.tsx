import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import type { ImportSummary } from "../api";
import { exportData, importData } from "../api";

interface Props {
  bump: () => void;
}

export default function BackupView({ bump }: Props) {
  const [exporting, setExporting] = useState(false);
  const [importing, setImporting] = useState(false);
  const [exportResult, setExportResult] = useState("");
  const [importResult, setImportResult] = useState<ImportSummary | null>(null);
  const [error, setError] = useState("");

  async function handleExport() {
    const dir = await open({ directory: true, title: "Choose a folder for the backup" });
    if (!dir || Array.isArray(dir)) return;
    setError("");
    setExporting(true);
    try {
      await exportData(dir);
      setExportResult(dir);
    } catch (err) {
      setError(String(err));
    } finally {
      setExporting(false);
    }
  }

  async function handleImport() {
    const dir = await open({ directory: true, title: "Choose a backup folder to restore from" });
    if (!dir || Array.isArray(dir)) return;
    setError("");
    setImportResult(null);
    setImporting(true);
    try {
      const summary = await importData(dir);
      setImportResult(summary);
      bump();
    } catch (err) {
      setError(String(err));
    } finally {
      setImporting(false);
    }
  }

  const cardStyle = { borderColor: "var(--border)", backgroundColor: "var(--surface-1)" };

  return (
    <div className="flex flex-col gap-6">
      <div className="rounded-lg border p-4" style={cardStyle}>
        <h3 className="font-medium">Export backup</h3>
        <p className="mt-1 text-sm" style={{ color: "var(--text-secondary)" }}>
          Writes every account, tag, transaction, subscription (with its billing history), and
          budget to CSV files in a folder you choose - a complete, restorable backup.
        </p>
        <button
          onClick={handleExport}
          disabled={exporting}
          className="mt-3 rounded px-3 py-1.5 text-sm font-medium"
          style={{ backgroundColor: "var(--accent)", color: "white" }}
        >
          {exporting ? "Exporting…" : "Choose folder & export"}
        </button>
        {exportResult && (
          <p className="mt-2 text-sm" style={{ color: "var(--status-good)" }}>
            Exported to {exportResult}
          </p>
        )}
      </div>

      <div className="rounded-lg border p-4" style={cardStyle}>
        <h3 className="font-medium">Restore from backup</h3>
        <p className="mt-1 text-sm" style={{ color: "var(--text-secondary)" }}>
          Restores from a folder created by Export. Meant for a fresh install with no existing
          data - if any of these accounts/transactions/etc. already exist, the restore will fail
          rather than duplicate or overwrite anything.
        </p>
        <button
          onClick={handleImport}
          disabled={importing}
          className="mt-3 rounded border px-3 py-1.5 text-sm font-medium"
          style={{ borderColor: "var(--border)" }}
        >
          {importing ? "Restoring…" : "Choose folder & restore"}
        </button>
        {importResult && (
          <p className="mt-2 text-sm" style={{ color: "var(--status-good)" }}>
            Restored {importResult.accounts} accounts, {importResult.tags} tags,{" "}
            {importResult.transactions} transactions, {importResult.subscriptions} subscriptions,{" "}
            {importResult.occurrences} billing cycles, {importResult.budgets} budgets.
          </p>
        )}
      </div>

      {error && (
        <p className="text-sm" style={{ color: "var(--status-critical)" }}>
          {error}
        </p>
      )}
    </div>
  );
}
