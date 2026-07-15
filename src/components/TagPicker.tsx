import { useState } from "react";
import type { Tag } from "../api";
import { createTag } from "../api";

interface Props {
  tags: Tag[];
  selected: number[];
  onChange: (ids: number[]) => void;
  onTagCreated: (tag: Tag) => void;
}

// These are seeded for cross-ledger money movement, not everyday categorization -
// keep them visually separate so they don't clutter the main tag row.
const SPECIAL_TAG_NAMES = new Set(["Owner's Draw", "Business Loan", "Loan Repayment"]);

export default function TagPicker({ tags, selected, onChange, onTagCreated }: Props) {
  const [newTagName, setNewTagName] = useState("");

  function toggle(id: number) {
    onChange(selected.includes(id) ? selected.filter((t) => t !== id) : [...selected, id]);
  }

  async function addTag() {
    const name = newTagName.trim();
    if (!name) return;
    const tag = await createTag(name, null);
    onTagCreated(tag);
    onChange([...selected, tag.id]);
    setNewTagName("");
  }

  const regularTags = tags.filter((t) => !SPECIAL_TAG_NAMES.has(t.name));
  const specialTags = tags.filter((t) => SPECIAL_TAG_NAMES.has(t.name));

  function renderChip(tag: Tag) {
    const isSelected = selected.includes(tag.id);
    return (
      <button
        type="button"
        key={tag.id}
        onClick={() => toggle(tag.id)}
        className="rounded-full border px-3 py-1 text-xs transition-colors"
        style={{
          borderColor: isSelected ? tag.color ?? "var(--accent)" : "var(--border)",
          backgroundColor: isSelected ? `${tag.color ?? "#3987e5"}22` : "transparent",
          color: isSelected ? "var(--text-primary)" : "var(--text-secondary)",
        }}
      >
        {tag.name}
      </button>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap items-center gap-2">
        {regularTags.map(renderChip)}
        <input
          value={newTagName}
          onChange={(e) => setNewTagName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              addTag();
            }
          }}
          placeholder="+ new tag"
          className="w-24 rounded-full border border-dashed px-3 py-1 text-xs"
          style={{ borderColor: "var(--border)", color: "var(--text-primary)" }}
        />
      </div>
      {specialTags.length > 0 && (
        <div className="flex flex-wrap items-center gap-2 border-t pt-2" style={{ borderColor: "var(--gridline)" }}>
          <span className="text-xs" style={{ color: "var(--text-muted)" }}>Cross-ledger:</span>
          {specialTags.map(renderChip)}
        </div>
      )}
    </div>
  );
}
