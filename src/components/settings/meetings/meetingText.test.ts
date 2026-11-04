import assert from "node:assert/strict";
import { meetingMarkdown, meetingPreview, parseMeeting } from "./meetingText";

const saved =
  "Others: Hello there. Can you hear me?\n\nYou: Yes, loud and clear.\n\nOthers: Great.";

const turns = parseMeeting(saved);
assert.equal(turns.length, 3);
assert.deepEqual(turns[0], {
  speaker: "Others",
  text: "Hello there. Can you hear me?",
});
assert.equal(turns[1].speaker, "You");

// Text from before labels existed still shows up.
assert.deepEqual(parseMeeting("Just some words"), [
  { speaker: null, text: "Just some words" },
]);
assert.deepEqual(parseMeeting(""), []);

assert.equal(
  meetingMarkdown("Meeting", turns, { you: "You", others: "Others" }),
  "# Meeting\n\n**Others:** Hello there. Can you hear me?\n\n**You:** Yes, loud and clear.\n\n**Others:** Great.\n",
);

assert.equal(meetingPreview(turns, 20), "Hello there. Can you…");
console.log("meetingText tests passed");
