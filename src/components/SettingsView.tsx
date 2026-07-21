import { useState } from "react";
import type { Tag } from "../api";
import { wipeAllData } from "../api";
import type { DateFormat } from "../utils";
import { CURRENCIES, DATE_FORMAT_OPTIONS } from "../utils";

export type Theme = "dark" | "light";

interface Props {
  theme: Theme;
  onThemeChange: (theme: Theme) => void;
  currencyCode: string;
  onCurrencyChange: (code: string) => void;
  dateFormat: DateFormat;
  onDateFormatChange: (format: DateFormat) => void;
  tags: Tag[];
  onWiped: () => void;
}

// Only used as a fallback confirmation word on a brand-new install with no categories yet to
// pick from - otherwise an existing tag name is used instead.
const FALLBACK_WORDS = ["turnip", "compass", "lighthouse", "marble", "willow", "canyon", "otter", "quartz"];

function randomConfirmWord(tags: Tag[]): string {
  const candidates = tags.length > 0 ? tags.map((t) => t.name) : FALLBACK_WORDS;
  return candidates[Math.floor(Math.random() * candidates.length)];
}

export default function SettingsView({
  theme,
  onThemeChange,
  currencyCode,
  onCurrencyChange,
  dateFormat,
  onDateFormatChange,
  tags,
  onWiped,
}: Props) {
  const selectStyle = { borderColor: "var(--border)" };

  const [confirming, setConfirming] = useState(false);
  const [confirmWord, setConfirmWord] = useState("");
  const [inputValue, setInputValue] = useState("");
  const [wiping, setWiping] = useState(false);
  const [error, setError] = useState("");
  const [done, setDone] = useState(false);

  function startWipe() {
    setConfirmWord(randomConfirmWord(tags));
    setInputValue("");
    setError("");
    setDone(false);
    setConfirming(true);
  }

  function cancelWipe() {
    setConfirming(false);
    setInputValue("");
  }

  async function confirmWipe() {
    setError("");
    setWiping(true);
    try {
      await wipeAllData();
      setConfirming(false);
      setDone(true);
      onWiped();
    } catch (err) {
      setError(String(err));
    } finally {
      setWiping(false);
    }
  }

  const matches = inputValue.trim().toLowerCase() === confirmWord.toLowerCase();

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-end gap-6">
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-muted)" }}>Theme</label>
          <div className="flex gap-2">
            {(["dark", "light"] as Theme[]).map((t) => (
              <button
                key={t}
                type="button"
                onClick={() => onThemeChange(t)}
                className="rounded px-3 py-1.5 text-sm font-medium capitalize"
                style={{
                  backgroundColor: theme === t ? "var(--accent)" : "transparent",
                  color: theme === t ? "white" : "var(--text-secondary)",
                  border: theme === t ? "none" : "1px solid var(--border)",
                }}
              >
                {t}
              </button>
            ))}
          </div>
        </div>

        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-muted)" }}>Currency</label>
          <select
            value={currencyCode}
            onChange={(e) => onCurrencyChange(e.target.value)}
            className="rounded border bg-transparent px-2 py-1 text-sm"
            style={selectStyle}
          >
            {CURRENCIES.map((c) => (
              <option key={c.code} value={c.code} style={{ color: "black" }}>{c.label}</option>
            ))}
          </select>
        </div>

        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-muted)" }}>Date format</label>
          <select
            value={dateFormat}
            onChange={(e) => onDateFormatChange(e.target.value as DateFormat)}
            className="rounded border bg-transparent px-2 py-1 text-sm"
            style={selectStyle}
          >
            {DATE_FORMAT_OPTIONS.map((o) => (
              <option key={o.value} value={o.value} style={{ color: "black" }}>{o.label}</option>
            ))}
          </select>
        </div>
      </div>

      <div className="rounded-lg border p-4" style={{ borderColor: "var(--status-critical)" }}>
        <h3 className="font-medium" style={{ color: "var(--status-critical)" }}>Danger zone</h3>
        <p className="mt-1 text-sm" style={{ color: "var(--text-secondary)" }}>
          Permanently deletes every account, category, transaction, recurring item, budget, and debt calculator
          entry. This cannot be undone - export a backup first if you're not sure.
        </p>

        {!confirming ? (
          <button
            onClick={startWipe}
            className="mt-3 rounded px-3 py-1.5 text-sm font-medium"
            style={{ backgroundColor: "var(--status-critical)", color: "white" }}
          >
            Delete everything
          </button>
        ) : (
          <div className="mt-3 flex flex-col gap-2">
            <p className="text-sm" style={{ color: "var(--text-secondary)" }}>
              Type <strong>{confirmWord}</strong> below to confirm.
            </p>
            <div className="flex flex-wrap items-center gap-2">
              <input
                autoFocus
                value={inputValue}
                onChange={(e) => setInputValue(e.target.value)}
                className="w-48 rounded border bg-transparent px-2 py-1 text-sm"
                style={selectStyle}
              />
              <button
                onClick={confirmWipe}
                disabled={!matches || wiping}
                className="rounded px-3 py-1.5 text-sm font-medium disabled:opacity-40"
                style={{ backgroundColor: "var(--status-critical)", color: "white" }}
              >
                {wiping ? "Deleting…" : "Yes, delete everything"}
              </button>
              <button onClick={cancelWipe} className="rounded border px-3 py-1.5 text-sm" style={selectStyle}>
                Cancel
              </button>
            </div>
          </div>
        )}

        {done && <p className="mt-3 text-sm" style={{ color: "var(--status-good)" }}>Everything has been deleted.</p>}
        {error && <p className="mt-3 text-sm" style={{ color: "var(--status-critical)" }}>{error}</p>}
      </div>
    </div>
  );
}
