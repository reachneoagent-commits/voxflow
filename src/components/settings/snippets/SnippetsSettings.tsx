import React, { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Pencil, Trash2 } from "lucide-react";
import { toast } from "sonner";
import type { Snippet } from "@/bindings";
import { useSettings } from "../../../hooks/useSettings";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { Switch } from "../../ui/Switch";
import { Textarea } from "../../ui/Textarea";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { HelpNote } from "../../ui/HelpNote";
import { ConfirmDelete } from "../../ui/ConfirmDelete";
import { ClipboardPhrase } from "./ClipboardPhrase";

// A fixed fallback so a missing setting never changes identity between renders.
const NONE: never[] = [];

const MAX_TRIGGER = 100;
const MAX_TEXT = 10_000;

const normalizeTrigger = (value: string) => value.trim().replace(/\s+/g, " ");

interface EditorProps {
  initial: Snippet | null;
  existing: Snippet[];
  onSave: (snippet: Snippet) => void;
  onCancel: () => void;
}

const SnippetEditor: React.FC<EditorProps> = ({
  initial,
  existing,
  onSave,
  onCancel,
}) => {
  const { t } = useTranslation();
  const [trigger, setTrigger] = useState(initial?.trigger ?? "");
  const [text, setText] = useState(initial?.text ?? "");

  const handleSave = () => {
    const cleanTrigger = normalizeTrigger(trigger);
    if (!/[\p{L}\p{N}]/u.test(cleanTrigger)) {
      toast.error(t("settings.snippets.errors.triggerRequired"));
      return;
    }
    if (!text.trim()) {
      toast.error(t("settings.snippets.errors.textRequired"));
      return;
    }
    const clash = existing.some(
      (snippet) =>
        snippet.id !== initial?.id &&
        snippet.trigger.toLowerCase() === cleanTrigger.toLowerCase(),
    );
    if (clash) {
      toast.error(
        t("settings.snippets.errors.duplicate", { trigger: cleanTrigger }),
      );
      return;
    }
    onSave({
      id: initial?.id ?? crypto.randomUUID(),
      trigger: cleanTrigger,
      text,
      enabled: initial?.enabled ?? true,
    });
  };

  return (
    <div className="px-4 py-3 space-y-3 border-t border-line first:border-t-0">
      <div className="space-y-1">
        <label className="text-sm font-medium" htmlFor="snippet-trigger">
          {t("settings.snippets.trigger.label")}
        </label>
        <Input
          id="snippet-trigger"
          className="w-full"
          autoFocus
          maxLength={MAX_TRIGGER}
          value={trigger}
          placeholder={t("settings.snippets.trigger.placeholder")}
          onChange={(e) => setTrigger(e.target.value)}
        />
      </div>
      <div className="space-y-1">
        <label className="text-sm font-medium" htmlFor="snippet-text">
          {t("settings.snippets.text.label")}
        </label>
        <Textarea
          id="snippet-text"
          className="w-full"
          variant="compact"
          maxLength={MAX_TEXT}
          value={text}
          placeholder={t("settings.snippets.text.placeholder")}
          onChange={(e) => setText(e.target.value)}
        />
      </div>
      <div className="flex justify-end gap-2">
        <Button variant="outline" size="md" onClick={onCancel}>
          {t("common.cancel")}
        </Button>
        <Button variant="primary" size="md" onClick={handleSave}>
          {t("common.save")}
        </Button>
      </div>
    </div>
  );
};

export const SnippetsSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();
  const snippets = getSetting("snippets") ?? NONE;
  const [query, setQuery] = useState("");
  // "new" opens a blank editor; an id edits that snippet in place.
  const [editing, setEditing] = useState<string | "new" | null>(null);

  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return snippets;
    return snippets.filter(
      (snippet) =>
        snippet.trigger.toLowerCase().includes(q) ||
        snippet.text.toLowerCase().includes(q),
    );
  }, [snippets, query]);

  const save = (snippet: Snippet) => {
    const next = snippets.some((s) => s.id === snippet.id)
      ? snippets.map((s) => (s.id === snippet.id ? snippet : s))
      : [...snippets, snippet];
    updateSetting("snippets", next);
    setEditing(null);
  };

  const toggle = (id: string, enabled: boolean) =>
    updateSetting(
      "snippets",
      snippets.map((s) => (s.id === id ? { ...s, enabled } : s)),
    );

  const [pendingId, setPendingId] = useState<string | null>(null);
  const pendingSnippet = snippets.find((s) => s.id === pendingId);

  const remove = (id: string) =>
    updateSetting(
      "snippets",
      snippets.filter((s) => s.id !== id),
    );

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <HelpNote
        title={t("settings.snippets.help.title")}
        example={t("settings.snippets.help.example")}
      >
        {t("settings.snippets.help.body")}
      </HelpNote>
      <ClipboardPhrase />
      <div className="flex items-center gap-2">
        <Input
          className="flex-1"
          type="search"
          value={query}
          placeholder={t("settings.snippets.search")}
          aria-label={t("settings.snippets.search")}
          onChange={(e) => setQuery(e.target.value)}
        />
        <Button
          variant="primary"
          size="md"
          onClick={() => setEditing("new")}
          disabled={editing === "new"}
        >
          {t("settings.snippets.new")}
        </Button>
      </div>

      <SettingsGroup
        title={t("settings.snippets.title")}
        description={t("settings.snippets.description")}
      >
        {editing === "new" && (
          <SnippetEditor
            initial={null}
            existing={snippets}
            onSave={save}
            onCancel={() => setEditing(null)}
          />
        )}
        {visible.length === 0 && editing !== "new" && (
          <p className="px-4 py-6 text-sm text-ink-2 text-center">
            {snippets.length === 0
              ? t("settings.snippets.empty")
              : t("settings.snippets.noResults")}
          </p>
        )}
        {visible.map((snippet) =>
          editing === snippet.id ? (
            <SnippetEditor
              key={snippet.id}
              initial={snippet}
              existing={snippets}
              onSave={save}
              onCancel={() => setEditing(null)}
            />
          ) : (
            <div
              key={snippet.id}
              className="flex items-center justify-between gap-3 px-4 py-2 min-h-12"
            >
              <div className="min-w-0 flex-1">
                <h3
                  className={`text-sm font-medium truncate ${snippet.enabled === false ? "opacity-50" : ""}`}
                >
                  {t("settings.snippets.triggerDisplay", {
                    trigger: snippet.trigger,
                  })}
                </h3>
                <p
                  className={`text-xs text-ink-2 truncate ${snippet.enabled === false ? "opacity-50" : ""}`}
                >
                  {snippet.text}
                </p>
              </div>
              <div className="flex items-center gap-1 shrink-0">
                <Button
                  variant="ghost"
                  size="sm"
                  aria-label={t("common.edit")}
                  onClick={() => setEditing(snippet.id)}
                >
                  <Pencil width={16} height={16} />
                </Button>
                <Button
                  variant="danger-ghost"
                  size="sm"
                  aria-label={t("common.delete")}
                  onClick={() => setPendingId(snippet.id)}
                >
                  <Trash2 width={16} height={16} />
                </Button>
                <Switch
                  checked={snippet.enabled ?? true}
                  ariaLabel={snippet.trigger}
                  onChange={(enabled) => toggle(snippet.id, enabled)}
                />
              </div>
            </div>
          ),
        )}
      </SettingsGroup>
      <ConfirmDelete
        name={pendingSnippet?.trigger ?? null}
        onCancel={() => setPendingId(null)}
        onConfirm={() => {
          if (pendingId) remove(pendingId);
          setPendingId(null);
        }}
      />
    </div>
  );
};
