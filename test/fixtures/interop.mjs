// Loads the document written by test/interop/write_fixtures.rb and checks JS sees the right
// values, then syncs a JS peer with the Ruby peer in test/interop/sync_step.rb.
import * as A from "@automerge/automerge";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const out = (name) => readFileSync(new URL(`../interop/out/${name}`, import.meta.url));
const doc = A.load(out("ruby.automerge"));

assert.ok(doc.string instanceof A.ImmutableString, "Ruby String is an ImmutableString");
assert.equal(String(doc.string), "plain");
assert.equal(doc.text, "héllo 😀");
assert.equal(doc.int, -3);
assert.equal(doc.float, 2.5);
assert.equal(doc.bool, true);
assert.equal(doc.nothing, null);
assert.ok(doc.counter instanceof A.Counter, "Counter");
assert.equal(doc.counter.value, 5);
assert.ok(doc.timestamp instanceof Date, "Timestamp is a Date");
assert.equal(doc.timestamp.getTime(), 1700000000123);
assert.equal(doc.uint, 7n);
assert.deepEqual(Array.from(doc.bytes), [0, 1, 255]);
assert.equal(doc.nested.list[0], 1);
assert.equal(String(doc.nested.list[1]), "two");
assert.equal(doc.nested.list[2].three, 3);
assert.deepEqual(A.getHeads(doc), JSON.parse(out("ruby.heads.json")), "Ruby heads match getHeads");

// Both peers start from types.automerge and make one change each. The JS peer stays in
// memory; the Ruby peer takes each turn in a fresh process that reloads its document and
// sync state from disk, as a server handling one message per request would.
const rubyTurn = (message) => {
  const script = fileURLToPath(new URL("../interop/sync_step.rb", import.meta.url));
  return execFileSync("ruby", [script], { input: message });
};
let js = A.change(A.load(readFileSync(new URL("types.automerge", import.meta.url))), (doc) => {
  doc.js = "from js";
});
let state = A.initSyncState();
let message;
for (let turn = 0; ; turn++) {
  assert.ok(turn < 10, "sync did not settle within 10 turns");
  [state, message] = A.generateSyncMessage(js, state);
  if (message === null) break;
  const reply = rubyTurn(message);
  if (reply.length > 0) [js, state] = A.receiveSyncMessage(js, state, reply);
}
assert.deepEqual(A.getHeads(js), JSON.parse(out("sync/ruby.heads.json")), "peers converge on the same heads");
assert.equal(js.ruby, "from ruby", "JS peer has the Ruby change");
assert.equal(A.load(out("sync/ruby.automerge")).js, "from js", "Ruby peer has the JS change");

console.log("interop ok");
