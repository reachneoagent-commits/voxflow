import React from "react";
import { useTranslation } from "react-i18next";
import { Dropdown } from "../ui/Dropdown";
import { SettingContainer } from "../ui/SettingContainer";
import { useSettings } from "../../hooks/useSettings";
import type { FocusStrength } from "@/bindings";

interface FocusOnMyVoiceProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

const STRENGTHS: { value: FocusStrength; key: string }[] = [
  { value: "off", key: "off" },
  { value: "gentle", key: "gentle" },
  { value: "normal", key: "normal" },
  { value: "strong", key: "strong" },
];

/** Ignores voices quieter than the person dictating, e.g. nearby talk. */
export const FocusOnMyVoice: React.FC<FocusOnMyVoiceProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const selected = (getSetting("focus_on_my_voice") ??
      "off") as FocusStrength;

    const options = STRENGTHS.map(({ value, key }) => ({
      value,
      label: t(`settings.sound.focusOnMyVoice.options.${key}`),
      description: t(`settings.sound.focusOnMyVoice.descriptions.${key}`),
    }));

    return (
      <SettingContainer
        title={t("settings.sound.focusOnMyVoice.title")}
        description={t("settings.sound.focusOnMyVoice.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      >
        <Dropdown
          options={options}
          menuClassName="right-0 w-80 max-w-[calc(100vw-2rem)]"
          selectedValue={selected}
          onSelect={(value) =>
            updateSetting("focus_on_my_voice", value as FocusStrength)
          }
          disabled={isUpdating("focus_on_my_voice")}
        />
      </SettingContainer>
    );
  },
);
