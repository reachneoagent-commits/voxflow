import React from "react";
import { Lightbulb } from "lucide-react";

interface HelpNoteProps {
  title: string;
  children: React.ReactNode;
  /** A worked example, shown set apart under the explanation. */
  example?: React.ReactNode;
}

/** A short "how this works" card for the top of a settings page. */
export const HelpNote: React.FC<HelpNoteProps> = ({
  title,
  children,
  example,
}) => (
  <div className="flex gap-3 rounded-lg border border-line bg-logo-primary/10 px-4 py-3">
    <Lightbulb
      className="mt-0.5 h-5 w-5 shrink-0 text-logo-primary"
      aria-hidden="true"
    />
    <div className="space-y-1.5 text-sm">
      <h2 className="font-semibold">{title}</h2>
      <div className="text-ink-2">{children}</div>
      {example && (
        <div className="rounded-md bg-background/60 px-3 py-2 text-sm">
          {example}
        </div>
      )}
    </div>
  </div>
);
