export interface MeetingTurn {
  /** "You", "Others", or null for text without a speaker label. */
  speaker: "You" | "Others" | null;
  text: string;
}

/** A meeting's saved text as one entry per change of speaker. */
export const parseMeeting = (text: string): MeetingTurn[] =>
  text
    .split(/\n{2,}/)
    .map((paragraph) => paragraph.trim())
    .filter(Boolean)
    .map((paragraph) => {
      const match = /^(You|Others):\s*([\s\S]*)$/.exec(paragraph);
      return match
        ? { speaker: match[1] as "You" | "Others", text: match[2].trim() }
        : { speaker: null, text: paragraph };
    });

/** The transcript as a Markdown document. */
export const meetingMarkdown = (
  title: string,
  turns: MeetingTurn[],
  labels: { you: string; others: string },
): string => {
  const body = turns
    .map((turn) => {
      const who =
        turn.speaker === "You"
          ? labels.you
          : turn.speaker === "Others"
            ? labels.others
            : null;
      return who ? `**${who}:** ${turn.text}` : turn.text;
    })
    .join("\n\n");
  return `# ${title}\n\n${body}\n`;
};

/** The first words of what was said, for the list. */
export const meetingPreview = (turns: MeetingTurn[], max = 110): string => {
  const text = turns.map((turn) => turn.text).join(" ");
  return text.length > max ? `${text.slice(0, max).trimEnd()}…` : text;
};
