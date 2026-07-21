import type { Account, DebtProgress, Tag } from "../api";
import type { DateFormat } from "../utils";
import AccountsView from "./AccountsView";
import BackupView from "./BackupView";
import DebtSnowballView from "./DebtSnowballView";
import SettingsView, { type Theme } from "./SettingsView";

interface Props {
  accounts: Account[];
  tags: Tag[];
  debts: DebtProgress[];
  bump: () => void;
  onAccountCreated: (account: Account) => void;
  theme: Theme;
  onThemeChange: (theme: Theme) => void;
  currencyCode: string;
  onCurrencyChange: (code: string) => void;
  dateFormat: DateFormat;
  onDateFormatChange: (format: DateFormat) => void;
}

export default function MiscView({
  accounts,
  tags,
  debts,
  bump,
  onAccountCreated,
  theme,
  onThemeChange,
  currencyCode,
  onCurrencyChange,
  dateFormat,
  onDateFormatChange,
}: Props) {
  return (
    <div className="flex flex-col gap-10">
      <section>
        <h2 className="mb-4 text-lg font-semibold">Accounts</h2>
        <AccountsView accounts={accounts} onAccountCreated={onAccountCreated} bump={bump} />
      </section>

      <section className="border-t pt-8" style={{ borderColor: "var(--gridline)" }}>
        <h2 className="mb-4 text-lg font-semibold">Debt Calculator</h2>
        <DebtSnowballView debts={debts} bump={bump} />
      </section>

      <section className="border-t pt-8" style={{ borderColor: "var(--gridline)" }}>
        <h2 className="mb-4 text-lg font-semibold">Settings</h2>
        <SettingsView
          theme={theme}
          onThemeChange={onThemeChange}
          currencyCode={currencyCode}
          onCurrencyChange={onCurrencyChange}
          dateFormat={dateFormat}
          onDateFormatChange={onDateFormatChange}
          tags={tags}
          onWiped={bump}
        />
      </section>

      <section className="border-t pt-8" style={{ borderColor: "var(--gridline)" }}>
        <h2 className="mb-4 text-lg font-semibold">Backup</h2>
        <BackupView bump={bump} />
      </section>
    </div>
  );
}
