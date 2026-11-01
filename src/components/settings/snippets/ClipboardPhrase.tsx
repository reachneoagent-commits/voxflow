import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { X } from "lucide-react";
import { toast } from "sonner";
import { useSettings } from "../../../hooks/useSettings";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { ToggleSwitch } from "../../ui/ToggleSwitch";

/** Mirror the backend defaults (settings.rs). */
const DEFAULT_PASTE_PHRASES = [
  "paste clipboard",
  "paste link",
  "paste url",
  "insert link",
  "insert clipboard",
  "past clipboard",
  "past link",
];
const DEFAULT_LINK_PHRASES = [
  "this link",
  "the link",
  "this url",
  "this video",
  "this article",
  "this page",
  "this website",
];

interface PhraseChipsProps {
  phrases: string[];
  defaults: string[];
  disabled: boolean;
  label: string;
  description: string;
  placeholder: string;
  onChange: (phrases: string[]) => void;
}

/** Removable phrase chips with an add box and a reset button. */
const PhraseChips: React.FC<PhraseChipsProps> = ({
  phrases,
  defaults,
  disabled,
  label,
  description,
  placeholder,
  onChange,
}) => {
  const { t } = useTranslation();
  const [draft, setDraft] = useState("");

  const add = () => {
    const next = draft.trim().replace(/\s+/g, " ");
    if (!next) return;
    if (!/[\p{L}\p{N}]/u.test(next)) {
      toast.error(t("settings.snippets.clipboard.phrases.error"));
      return;
    }
    if (phrases.some((p) => p.toLowerCase() === next.toLowerCase())) {
      toast.error(
        t("settings.snippets.clipboard.phrases.duplicate", { phrase: next }),
      );
      return;
    }
    onChange([...phrases, next]);
    setDraft("");
  };

  return (
    <div className={`px-4 py-3 space-y-3 ${disabled ? "opacity-50" : ""}`}>
      <div>
        <h3 className="text-sm font-medium">{label}</h3>
        <p className="text-xs text-ink-2">{description}</p>
      </div>
      <ul className="flex flex-wrap gap-2">
        {phrases.map((phrase) => (
          <li
            key={phrase}
            className="flex items-center gap-1 rounded-full bg-logo-primary/20 py-1 ps-3 pe-1 text-sm"
          >
            {phrase}
            <button
              type="button"
              disabled={disabled}
              aria-label={t("common.delete")}
              onClick={() => onChange(phrases.filter((p) => p !== phrase))}
              className="grid size-5 place-items-center rounded-full text-ink-2 hover:bg-wash hover:text-text cursor-pointer disabled:cursor-default"
            >
              <X width={14} height={14} />
            </button>
          </li>
        ))}
      </ul>
      <div className="flex items-center gap-2">
        <Input
          className="flex-1"
          value={draft}
          maxLength={50}
          disabled={disabled}
          placeholder={placeholder}
          aria-label={placeholder}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && add()}
        />
        <Button
          variant="primary"
          size="md"
          onClick={add}
          disabled={disabled || !draft.trim()}
        >
          {t("settings.snippets.clipboard.phrases.add")}
        </Button>
        <Button
          variant="outline"
          size="md"
          disabled={disabled}
          onClick={() => onChange(defaults)}
        >
          {t("settings.snippets.clipboard.phrases.reset")}
        </Button>
      </div>
    </div>
  );
};

/** Type what you copied: say a paste phrase, or say "this link" to add a copied link. */
export const ClipboardPhrase: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const enabled = getSetting("clipboard_phrase_enabled") ?? true;
  const phrases = getSetting("clipboard_phrases") ?? DEFAULT_PASTE_PHRASES;
  const linkEnabled = getSetting("clipboard_link_enabled") ?? true;
  const linkPhrases =
    getSetting("clipboard_link_phrases") ?? DEFAULT_LINK_PHRASES;

  return (
    <SettingsGroup
      title={t("settings.snippets.clipboard.title")}
      description={t("settings.snippets.clipboard.description")}
    >
      <ToggleSwitch
        checked={linkEnabled}
        onChange={(value) => updateSetting("clipboard_link_enabled", value)}
        isUpdating={isUpdating("clipboard_link_enabled")}
        label={t("settings.snippets.clipboard.link.toggle.label")}
        description={t("settings.snippets.clipboard.link.toggle.description")}
        descriptionMode="tooltip"
        grouped={true}
      />
      <PhraseChips
        phrases={linkPhrases}
        defaults={DEFAULT_LINK_PHRASES}
        disabled={!linkEnabled}
        label={t("settings.snippets.clipboard.link.phrases.label")}
        description={t("settings.snippets.clipboard.link.phrases.description")}
        placeholder={t("settings.snippets.clipboard.link.phrases.placeholder")}
        onChange={(next) => updateSetting("clipboard_link_phrases", next)}
      />
      <ToggleSwitch
        checked={enabled}
        onChange={(value) => updateSetting("clipboard_phrase_enabled", value)}
        isUpdating={isUpdating("clipboard_phrase_enabled")}
        label={t("settings.snippets.clipboard.toggle.label")}
        description={t("settings.snippets.clipboard.toggle.description")}
        descriptionMode="tooltip"
        grouped={true}
      />
      <PhraseChips
        phrases={phrases}
        defaults={DEFAULT_PASTE_PHRASES}
        disabled={!enabled}
        label={t("settings.snippets.clipboard.phrases.label")}
        description={t("settings.snippets.clipboard.phrases.description")}
        placeholder={t("settings.snippets.clipboard.phrases.placeholder")}
        onChange={(next) => updateSetting("clipboard_phrases", next)}
      />
    </SettingsGroup>
  );
};
